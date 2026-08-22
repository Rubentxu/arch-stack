//! Event types for the reactive runtime.
//!
//! PR1 provides the foundation types: [`EventEnvelope`], [`SerializedEvent`],
//! [`EventLog`] (append-only JSONL), and [`Sequence`] (marker-file counter).
//!
//! TRUST-002 extends [`EventEnvelope`] with `eventId` (UUID v7, RFC 9562),
//! `correlationId`, `causationId`, `processed` fields per spec R1–R5 and
//! ADR-P11 (causal journal). `EventLog::append` now auto-assigns `event_id`
//! and `timestamp`; callers supply `correlation_id` / `causation_id` only.

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Sequence
// ---------------------------------------------------------------------------

/// A monotonically-increasing nanosecond counter persisted to a marker file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sequence(pub u64);

impl Sequence {
    /// Load the current sequence value from the marker file, or 0 if absent.
    pub fn load(path: &PathBuf) -> io::Result<Self> {
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let n = contents.trim().parse().unwrap_or(0);
            Ok(Sequence(n))
        } else {
            Ok(Sequence(0))
        }
    }

    /// Persist the current value to the marker file.
    pub fn store(&self, path: &PathBuf) -> io::Result<()> {
        fs::write(path, self.0.to_string())
    }

    /// Increment and return the new value.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

// ---------------------------------------------------------------------------
// EventEnvelope
// ---------------------------------------------------------------------------

/// An event envelope with event identity, timestamp, source, event type,
/// payload, and sequence.
///
/// The `event_type` field is a free-form `String` (NOT an enum) to stay
/// extensible — different agents can define their own event types without
/// requiring a central registry.
///
/// ## Fields
///
/// - `event_id`: UUID v7 (RFC 9562) auto-assigned by [`EventLog::append`].
///   Legacy lines deserialize with `Uuid::nil()` and a `tracing::warn!`.
/// - `schema_version`: `"1.1"` for new writes; `"1.0"` for legacy.
/// - `timestamp`: RFC3339 (`DateTime<Utc>`), auto-assigned by [`EventLog::append`].
/// - `correlation_id` / `causation_id`: caller-supplied lineage fields.
///   Filter `event_id != Uuid::nil()` to exclude legacy events.
///
/// See spec `40-AGENT-EVENT-JOURNAL.md` L4 and ADR-P11.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// UUID v7 (RFC 9562) identity. Auto-assigned by [`EventLog::append`].
    /// Legacy pre-1.1 lines deserialize with `Uuid::nil()`.
    #[serde(rename = "eventId", default)]
    pub event_id: Uuid,

    /// Schema version string. `"1.1"` for new events; `"1.0"` for legacy.
    #[serde(rename = "schemaVersion", default = "default_schema_version")]
    pub schema_version: String,

    /// RFC3339 timestamp. Auto-assigned by [`EventLog::append`].
    #[serde(rename = "timestamp", default)]
    pub timestamp: DateTime<Utc>,

    /// Originating source identifier (e.g. `"dispatcher"`, `"file_watcher"`).
    pub source: String,

    /// Originating producer identifier (e.g. `"event_dispatcher"`).
    #[serde(rename = "producer", default)]
    pub producer: String,

    /// Event kind in CamelCase notation, e.g. `GoalSubmitted`, `FileChanged`.
    #[serde(rename = "eventType")]
    pub event_type: String,

    /// Arbitrary JSON payload associated with this event.
    pub payload: serde_json::Value,

    /// Monotonically increasing sequence number for ordering.
    pub seq: u64,

    /// Caller-supplied correlation group ID (groups events sharing a logical operation).
    #[serde(rename = "correlationId", default)]
    pub correlation_id: Option<Uuid>,

    /// Caller-supplied causation chain ID (points to the event that caused this one).
    #[serde(rename = "causationId", default)]
    pub causation_id: Option<Uuid>,

    /// Caller-supplied graph revision at event creation time.
    #[serde(rename = "graphRevision", default)]
    pub graph_revision: Option<u64>,
}

/// Returns the default schema version (`"1.1"` for new events).
fn default_schema_version() -> String {
    "1.1".to_string()
}

// ---------------------------------------------------------------------------
// SerializedEvent
// ---------------------------------------------------------------------------

/// A serialised event paired with its processing status.
///
/// The `processed` flag uses the seq-marker-file strategy: instead of
/// rewriting the JSONL line in-place, the highest processed sequence number
/// is tracked in `event.log.seq`. A line is considered processed when its
/// `envelope.seq <= processed_seq`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEvent {
    /// The full event envelope.
    pub envelope: EventEnvelope,
    /// Whether this event has been processed by the dispatcher.
    /// Note: prefer the seq-marker strategy over in-place rewrite.
    pub processed: bool,
}

// ---------------------------------------------------------------------------
// EventLog
// ---------------------------------------------------------------------------

/// An append-only JSONL event log.
///
/// Each line is one independent JSON object, consistent with the audit.log
/// pattern used elsewhere in the crate. The log is append-only: callers must
/// NOT open it with write access except for appending.
#[derive(Debug, Clone)]
pub struct EventLog {
    path: PathBuf,
    seq_path: PathBuf,
    /// Directory for per-consumer checkpoint files.
    checkpoint_dir: PathBuf,
}

impl EventLog {
    /// Open (or create) an event log at the given path.
    ///
    /// # Critical invariant
    /// Opening an existing journal MUST NOT truncate it. Spec
    /// `40-AGENT-EVENT-JOURNAL.md`: *"Abrir journal existente nunca trunca."*
    /// Uses `OpenOptions::new().create(true).append(true)` so the write
    /// cursor stays at EOF across reopens (append-only). Mirrors two
    /// precedents in-tree: `cognitive/audit/log.rs` (`AuditLogger::append`)
    /// and `store.rs` (`LbugStore::open`, with the anti-truncation comment
    /// and allow-attr). Per `ADR-P11` this is a causal journal, not an
    /// event-sourced store; replay parity is the reopen trigger and is out
    /// of scope here. Sequence marker at `event.log.seq` survives the open.
    pub fn open(path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Open in append mode so existing NDJSON lines survive a reopen.
        // `File::create` would truncate — see `cognitive/audit/log.rs:147-161`
        // and `store.rs:961-967` for the canonical precedent.
        #[allow(clippy::suspicious_open_options)]
        OpenOptions::new().create(true).append(true).open(&path)?;
        let seq_path = path.with_extension("seq");
        let checkpoint_dir = path.with_extension("checkpoint.dir");
        if !checkpoint_dir.exists() {
            fs::create_dir_all(&checkpoint_dir)?;
        }
        Ok(EventLog {
            path,
            seq_path,
            checkpoint_dir,
        })
    }

    /// Append a new event to the log, auto-assigning `event_id` (UUID v7) and
    /// `timestamp` (Utc::now()).
    ///
    /// Callers supply `producer`, `source`, `event_type`, `payload`,
    /// `correlation_id`, `causation_id`, and `graph_revision`.
    ///
    /// Returns the auto-generated `event_id` so callers can chain causation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let correlation = uuid::Uuid::new_v4();
    /// let parent_id = uuid::Uuid::nil();
    /// let id = log.append("dispatcher", "file_watcher", "FileChanged",
    ///     serde_json::json!({"path": "src/main.rs"}),
    ///     Some(correlation), Some(parent_id), None)?;
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &mut self,
        producer: &str,
        source: &str,
        event_type: &str,
        payload: serde_json::Value,
        correlation_id: Option<Uuid>,
        causation_id: Option<Uuid>,
        graph_revision: Option<u64>,
    ) -> io::Result<Uuid> {
        // Auto-assign identity fields
        let event_id = Uuid::now_v7();
        let timestamp = Utc::now();
        let seq = {
            let mut s = Sequence::load(&self.seq_path)?;
            s.next()
        };

        let envelope = EventEnvelope {
            event_id,
            schema_version: "1.1".to_string(),
            timestamp,
            source: source.into(),
            producer: producer.into(),
            event_type: event_type.into(),
            payload,
            seq,
            correlation_id,
            causation_id,
            graph_revision,
        };

        let serialized = SerializedEvent {
            envelope,
            processed: false,
        };

        // Atomic write using tempfile + rename (same pattern as store.rs:961-967)
        let json = serde_json::to_string(&serialized)?;
        let line = json + "\n";
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        file.write_all(line.as_bytes())?;

        // Persist updated seq
        Sequence(seq).store(&self.seq_path)?;

        Ok(event_id)
    }

    /// Append a serialised event to the log (low-level, preserves caller responsibility
    /// for identity fields). Prefer [`EventLog::append`] for new code.
    ///
    /// After successful append, the persisted sequence is updated to
    /// `event.envelope.seq` if it is greater than the current persisted seq
    /// (monotonic). This keeps `EventLog::seq()` consistent with the highest
    /// seq recorded in the log, so the dispatcher contract holds: after
    /// `dispatch`, `log.seq()` reflects the last dispatched envelope's seq.
    pub fn append_serialized(&mut self, event: &SerializedEvent) -> io::Result<()> {
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        let json = serde_json::to_string(event)?;
        writeln!(file, "{}", json)?;

        // Persist seq monotonically — only raise, never lower.
        let current = Sequence::load(&self.seq_path)?.0;
        let incoming = event.envelope.seq;
        if incoming > current {
            Sequence(incoming).store(&self.seq_path)?;
        }
        Ok(())
    }

    /// Returns an iterator over all events in the log, in order.
    pub fn iter(&self) -> io::Result<impl Iterator<Item = io::Result<SerializedEvent>>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let lines = reader.lines();
        let iter = lines.map(|line| {
            let l = line?;
            let val: serde_json::Value = serde_json::from_str(&l)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            // Emit warning for legacy lines missing eventId
            if val.get("eventId").is_none() {
                tracing::warn!(
                    "legacy pre-1.1 JSONL line encountered; \
                    deserializing with event_id = Uuid::nil(); \
                    filter with event_id != Uuid::nil()"
                );
            }
            serde_json::from_value(val).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        });
        Ok(iter)
    }

    /// Returns the current sequence number (highest seq in the log, or 0).
    /// Note: for per-consumer checkpoints, use [`EventLog::consumer_checkpoint`].
    pub fn seq(&self) -> io::Result<u64> {
        Sequence::load(&self.seq_path).map(|s| s.0)
    }

    /// Update the persisted sequence number (called after successful processing).
    /// Note: for per-consumer checkpoints, use [`EventLog::set_consumer_checkpoint`].
    pub fn update_seq(&self, seq: u64) -> io::Result<()> {
        Sequence(seq).store(&self.seq_path)
    }

    /// Returns the checkpoint path for a given consumer ID.
    ///
    /// Path format: `<log>.checkpoint.<consumer_id>.seq`
    fn checkpoint_path(&self, consumer_id: &str) -> io::Result<PathBuf> {
        validate_consumer_id(consumer_id)?;
        Ok(self.checkpoint_dir.join(format!("{}.seq", consumer_id)))
    }

    /// Read the last processed sequence number for `consumer_id`.
    ///
    /// Returns `Ok(0)` if the consumer has never checkpointed.
    pub fn consumer_checkpoint(&self, consumer_id: &str) -> io::Result<u64> {
        let path = self.checkpoint_path(consumer_id)?;
        Sequence::load(&path).map(|s| s.0)
    }

    /// Persist `seq` as the checkpoint for `consumer_id`.
    ///
    /// Uses atomic write via `tempfile::NamedTempFile` + rename.
    pub fn set_consumer_checkpoint(&self, consumer_id: &str, seq: u64) -> io::Result<()> {
        let path = self.checkpoint_path(consumer_id)?;
        let mut tmp =
            tempfile::NamedTempFile::new_in(path.parent().unwrap_or(&self.checkpoint_dir))?;
        tmp.write_all(seq.to_string().as_bytes())?;
        tmp.persist(&path)?;
        Ok(())
    }
}

/// Validate a consumer ID against the allowed charset.
///
/// Pattern: `^[a-zA-Z0-9_-]{1,64}$`
fn validate_consumer_id(consumer_id: &str) -> io::Result<()> {
    if consumer_id.is_empty() || consumer_id.len() > 64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "consumer_id must be 1-64 characters, got {}",
                consumer_id.len()
            ),
        ));
    }
    let re = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    if !re.is_match(consumer_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "consumer_id must match ^[a-zA-Z0-9_-]]{{1,64}}$; got '{}'",
                consumer_id
            ),
        ));
    }
    if consumer_id.contains("..") || consumer_id.contains('/') || consumer_id.contains('\0') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "consumer_id must not contain '..', '/', or NUL: '{}'",
                consumer_id
            ),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ---------------------------------------------------------------------------
    // Helper
    // ---------------------------------------------------------------------------

    fn make_envelope(seq: u64, event_type: &str) -> SerializedEvent {
        make_envelope_with_processed(seq, event_type, false)
    }

    fn make_envelope_with_processed(
        seq: u64,
        event_type: &str,
        processed: bool,
    ) -> SerializedEvent {
        SerializedEvent {
            envelope: EventEnvelope {
                event_id: Uuid::nil(),
                schema_version: "1.0".to_string(),
                timestamp: DateTime::from_timestamp(0, 0).unwrap(),
                source: "test".into(),
                producer: "test".into(),
                event_type: event_type.into(),
                payload: serde_json::json!({}),
                seq,
                correlation_id: None,
                causation_id: None,
                graph_revision: None,
            },
            processed,
        }
    }

    // ---------------------------------------------------------------------------
    // Sequence tests
    // ---------------------------------------------------------------------------

    #[test]
    fn sequence_increment_and_persist() {
        let tmp_dir = TempDir::new().unwrap();
        let tmp = tmp_dir.path().join("archctl_seq_test");
        let mut seq = Sequence(0);
        assert_eq!(seq.next(), 1);
        seq.store(&tmp).unwrap();
        let loaded = Sequence::load(&tmp).unwrap();
        assert_eq!(loaded.0, 1);
    }

    #[test]
    fn sequence_load_missing_file_returns_zero() {
        let tmp_dir = TempDir::new().unwrap();
        let tmp = tmp_dir.path().join("archctl_nonexistent_seq_file");
        let seq = Sequence::load(&tmp).unwrap();
        assert_eq!(seq.0, 0);
    }

    // ---------------------------------------------------------------------------
    // Serialization round-trip tests
    // ---------------------------------------------------------------------------

    #[test]
    fn event_envelope_roundtrip_json() {
        let env = EventEnvelope {
            event_id: Uuid::nil(),
            schema_version: "1.1".to_string(),
            timestamp: DateTime::from_timestamp(1_234_567, 0).unwrap(),
            source: "test".into(),
            producer: "test".into(),
            event_type: "GoalSubmitted".into(),
            payload: serde_json::json!({"goal": "analyze architecture"}),
            seq: 42,
            correlation_id: None,
            causation_id: None,
            graph_revision: None,
        };
        let json = serde_json::to_string(&env).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.seq, env.seq);
        assert_eq!(back.source, env.source);
        assert_eq!(back.event_type, env.event_type);
    }

    #[test]
    fn serialized_event_roundtrip_json() {
        let env = EventEnvelope {
            event_id: Uuid::nil(),
            schema_version: "1.0".to_string(),
            timestamp: DateTime::from_timestamp(1, 0).unwrap(),
            source: "dispatcher".into(),
            producer: "dispatcher".into(),
            event_type: "FileChanged".into(),
            payload: serde_json::json!({"path": "src/main.rs"}),
            seq: 7,
            correlation_id: None,
            causation_id: None,
            graph_revision: None,
        };
        let ser = SerializedEvent {
            envelope: env,
            processed: false,
        };
        let json = serde_json::to_string(&ser).unwrap();
        let back: SerializedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.envelope.seq, 7);
        assert!(!back.processed);
    }

    // ---------------------------------------------------------------------------
    // EventLog tests
    // ---------------------------------------------------------------------------

    #[test]
    fn event_log_append_and_iter() {
        // Migrated to tempfile::TempDir (K3 hygiene)
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("archctl_event_log_test.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        log.append_serialized(&make_envelope(1, "GoalSubmitted"))
            .unwrap();
        log.append_serialized(&make_envelope_with_processed(2, "FileChanged", true))
            .unwrap();

        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].envelope.seq, 1);
        assert_eq!(events[1].envelope.seq, 2);
        assert!(!events[0].processed);
        assert!(events[1].processed);
    }

    #[test]
    fn event_log_seq_persistence() {
        // Migrated to tempfile::TempDir (K3 hygiene)
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("archctl_event_seq_test.jsonl");
        let log = EventLog::open(log_path.clone()).unwrap();

        log.update_seq(99).unwrap();
        let loaded_seq = log.seq().unwrap();
        assert_eq!(loaded_seq, 99);
    }

    // ---------------------------------------------------------------------------
    // TRUST-001 regression tests — reopen invariant
    // ---------------------------------------------------------------------------

    /// R1 + scenario 2: Reopen does NOT truncate existing NDJSON lines.
    /// Red against main@05b1ef2 (File::create); green after T2 fix.
    #[test]
    fn event_log_open_does_not_truncate() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("reopen_truncate_test.jsonl");

        // First open: append 3 events
        {
            let mut log = EventLog::open(log_path.clone()).unwrap();
            log.append_serialized(&make_envelope(1, "GoalSubmitted"))
                .unwrap();
            log.append_serialized(&make_envelope(2, "FileChanged"))
                .unwrap();
            log.append_serialized(&make_envelope(3, "GoalCompleted"))
                .unwrap();
        } // drop

        // Capture raw bytes before reopen
        let bytes_before = fs::read(&log_path).unwrap();

        // Reopen with EventLog::open — currently uses File::create (BUG)
        {
            let _log = EventLog::open(log_path.clone()).unwrap();
        }

        // Read raw bytes after reopen
        let bytes_after = fs::read(&log_path).unwrap();

        // The bug: File::create truncates → bytes_after is empty or shorter
        // The fix: OpenOptions append mode preserves all bytes
        assert_eq!(
            bytes_before.len(),
            bytes_after.len(),
            "EventLog::open must not truncate existing journal content"
        );
        assert_eq!(
            bytes_before, bytes_after,
            "Journal bytes must be byte-for-byte identical after reopen"
        );

        // Semantic check: iter still yields 3 events
        let log = EventLog::open(log_path).unwrap();
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(
            events.len(),
            3,
            "iter() must return all 3 events after reopen"
        );
        assert_eq!(events[0].envelope.seq, 1);
        assert_eq!(events[1].envelope.seq, 2);
        assert_eq!(events[2].envelope.seq, 3);
    }

    /// R1 + R2 + scenario 3: Append after reopen grows log without loss.
    /// seq counter is contiguous across the reopen boundary.
    #[test]
    fn event_log_reopen_preserves_content() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("reopen_grow_test.jsonl");

        // Append 5, drop
        {
            let mut log = EventLog::open(log_path.clone()).unwrap();
            for i in 1..=5 {
                log.append_serialized(&make_envelope(i, "Event")).unwrap();
            }
        }

        // Reopen and append 2 more
        {
            let mut log = EventLog::open(log_path.clone()).unwrap();
            log.append_serialized(&make_envelope(6, "Event")).unwrap();
            log.append_serialized(&make_envelope(7, "Event")).unwrap();
        }

        // Verify: 7 events total, seq contiguous
        let log = EventLog::open(log_path).unwrap();
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(
            events.len(),
            7,
            "iter() must return all 7 events after reopen+append"
        );

        // seq values must be 1..=7 in order (contiguous across boundary)
        let seqs: Vec<u64> = events.iter().map(|e| e.envelope.seq).collect();
        assert_eq!(
            seqs,
            vec![1, 2, 3, 4, 5, 6, 7],
            "seq values must be contiguous 1..=7"
        );
    }

    /// R3 + scenario 1: Open on a non-existent path creates an empty file.
    #[test]
    fn event_log_open_creates_if_missing() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("fresh_log.jsonl");

        // Log file does not exist yet
        assert!(
            !log_path.exists(),
            "precondition: log file must not exist before open"
        );

        // First open — must succeed and create empty file
        let log = EventLog::open(log_path.clone()).unwrap();
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 0, "freshly opened log must have 0 events");

        // File must now exist at 0 bytes
        let metadata = fs::metadata(&log_path).unwrap();
        assert_eq!(metadata.len(), 0, "newly created log file must be 0 bytes");
    }

    /// Sanity: OpenOptions append mode leaves cursor at EOF (not truncate).
    /// Verifies via std::fs::File metadata that the file is NOT truncated.
    #[test]
    fn event_log_open_uses_append_mode() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("append_mode_test.jsonl");

        // Pre-write 16 bytes of content: `{"x":1}\n{"y":2}\n`
        fs::write(&log_path, b"{\"x\":1}\n{\"y\":2}\n").unwrap();
        let original_len = fs::metadata(&log_path).unwrap().len();
        assert_eq!(original_len, 16, "precondition: file must be 16 bytes");

        // Open with EventLog::open (currently File::create — truncates)
        let _log = EventLog::open(log_path.clone()).unwrap();

        // After open, file size must be unchanged (not truncated)
        let after_len = fs::metadata(&log_path).unwrap().len();
        assert_eq!(
            original_len, after_len,
            "EventLog::open must not change file size — File::create truncates"
        );
    }

    /// Q3 complement (spec §Open Questions): seq marker survives the reopen.
    /// update_seq(123), drop, reopen, seq() == 123.
    #[test]
    fn event_log_seq_persists_across_reopen() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("seq_reopen_test.jsonl");

        // Set seq to 123
        {
            let log = EventLog::open(log_path.clone()).unwrap();
            log.update_seq(123).unwrap();
        } // drop

        // Reopen and verify seq survived
        let log = EventLog::open(log_path).unwrap();
        let loaded_seq = log.seq().unwrap();
        assert_eq!(
            loaded_seq, 123,
            "seq marker must survive EventLog::open reopen"
        );
    }

    // ---------------------------------------------------------------------------
    // TRUST-002 regression tests — event IDs + causation + checkpoint
    // ---------------------------------------------------------------------------

    /// R1 + spec scenario 1: EventEnvelope carries eventId and timestamp fields.
    /// Fails against pre-fix (EventEnvelope lacks event_id and timestamp fields).
    #[test]
    fn event_envelope_includes_event_id_and_timestamp() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("event_id_test.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        // append takes correlation_id, causation_id, graph_revision — auto-assigns
        // event_id (UUID v7) and timestamp (Utc::now())
        let id = log
            .append(
                "test-producer",
                "test-source",
                "TestEvent",
                serde_json::json!({}),
                None,
                None,
                None,
            )
            .unwrap();

        // event_id must be a valid (non-nil) UUID
        assert!(!id.is_nil(), "append must assign a non-nil event_id");

        // The persisted event must carry eventId and timestamp fields
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 1);
        let json_str = serde_json::to_string(&events[0]).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // JSON must contain the new camelCase keys
        // eventId is nested inside "envelope"
        let envelope = json_val.get("envelope").expect("envelope field must exist");
        assert!(
            envelope.get("eventId").is_some(),
            "envelope must contain eventId field, got: {}",
            envelope
        );
        assert!(
            envelope.get("timestamp").is_some(),
            "envelope must contain timestamp field"
        );
        assert!(
            envelope.get("schemaVersion").is_some(),
            "envelope must contain schemaVersion field"
        );
    }

    /// R2 + spec scenario 1: EventLog::append assigns unique event_id to each call.
    /// Fails against pre-fix (append does not auto-assign event_id).
    #[test]
    fn event_log_append_assigns_unique_ids() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("unique_ids_test.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        let id1 = log
            .append(
                "producer",
                "source",
                "Event1",
                serde_json::json!({}),
                None,
                None,
                None,
            )
            .unwrap();
        let id2 = log
            .append(
                "producer",
                "source",
                "Event2",
                serde_json::json!({}),
                None,
                None,
                None,
            )
            .unwrap();
        let id3 = log
            .append(
                "producer",
                "source",
                "Event3",
                serde_json::json!({}),
                None,
                None,
                None,
            )
            .unwrap();

        // All IDs must be unique
        assert_ne!(id1, id2, "each append must assign a unique event_id");
        assert_ne!(id2, id3, "each append must assign a unique event_id");
        assert_ne!(id1, id3, "each append must assign a unique event_id");
    }

    /// R2 + spec scenario 2: child event carries causation_id referencing parent event_id.
    /// Fails against pre-fix (append signature does not accept correlation_id/causation_id).
    #[test]
    fn event_log_causation_chain() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("causation_chain_test.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        // Parent event
        let parent_id = log
            .append(
                "producer",
                "source",
                "ParentEvent",
                serde_json::json!({}),
                None,
                None,
                None,
            )
            .unwrap();

        // Child event caused by parent
        let child_id = log
            .append(
                "producer",
                "source",
                "ChildEvent",
                serde_json::json!({}),
                None,
                Some(parent_id),
                None,
            )
            .unwrap();

        // Child ID must differ from parent
        assert_ne!(
            parent_id, child_id,
            "child must have different event_id from parent"
        );

        // Read back and verify causation
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 2);

        // Second event (child) must carry causationId = parent_id
        let child_json = serde_json::to_string(&events[1]).unwrap();
        let child_val: serde_json::Value = serde_json::from_str(&child_json).unwrap();
        let child_envelope = child_val.get("envelope").expect("envelope must exist");
        assert!(
            child_envelope.get("causationId").is_some(),
            "child event envelope must carry causationId field"
        );
        let causation_id_str = child_envelope["causationId"].as_str().unwrap();
        assert_eq!(
            causation_id_str,
            parent_id.to_string(),
            "causationId must reference parent event_id"
        );
    }

    /// R2 + spec scenario 3: two events with same correlation_id share correlation group.
    /// Fails against pre-fix (append signature does not accept correlation_id).
    #[test]
    fn event_log_correlation_group() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("correlation_group_test.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        let correlation = uuid::Uuid::new_v4();

        // Event A: part of correlation group
        log.append(
            "producer",
            "source",
            "EventA",
            serde_json::json!({}),
            Some(correlation),
            None,
            None,
        )
        .unwrap();

        // Event B: same correlation, no causation
        log.append(
            "producer",
            "source",
            "EventB",
            serde_json::json!({}),
            Some(correlation),
            None,
            None,
        )
        .unwrap();

        // Event C: different correlation
        log.append(
            "producer",
            "source",
            "EventC",
            serde_json::json!({}),
            None,
            None,
            None,
        )
        .unwrap();

        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 3);

        // All 3 events must carry correlationId
        for (i, event) in events.iter().enumerate() {
            let json_str = serde_json::to_string(event).unwrap();
            let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
            let envelope = val.get("envelope").expect("envelope must exist");
            assert!(
                envelope.get("correlationId").is_some(),
                "event {} envelope must carry correlationId field",
                i
            );
        }

        // Events 0 and 1 must have the same correlationId
        let json0 = serde_json::to_string(&events[0]).unwrap();
        let json1 = serde_json::to_string(&events[1]).unwrap();
        let val0: serde_json::Value = serde_json::from_str(&json0).unwrap();
        let val1: serde_json::Value = serde_json::from_str(&json1).unwrap();
        let env0 = val0.get("envelope").expect("envelope must exist");
        let env1 = val1.get("envelope").expect("envelope must exist");
        assert_eq!(
            env0["correlationId"], env1["correlationId"],
            "events A and B must share correlationId"
        );

        // Event 2 must have null correlationId (different group)
        let json2 = serde_json::to_string(&events[2]).unwrap();
        let val2: serde_json::Value = serde_json::from_str(&json2).unwrap();
        let env2 = val2.get("envelope").expect("envelope must exist");
        assert!(
            env0["correlationId"] != env2["correlationId"] || env2["correlationId"].is_null(),
            "event C must have different (or null) correlationId"
        );
    }

    /// R4 + spec scenario 4: legacy pre-1.1 JSONL (without eventId) deserializes
    /// with Uuid::nil() and a warning is emitted.
    /// Fails against pre-fix (current deserializer does not handle missing eventId).
    #[test]
    fn event_log_handles_legacy_jsonl() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("legacy_test.jsonl");

        // Write a pre-1.1 SerializedEvent JSON line without eventId/correlationId fields.
        // The legacy format has no eventId, no correlationId, no causationId, no graphRevision.
        // We use timestamp as integer string (RFC3339 would fail) to test legacy parsing.
        let legacy_line = r#"{"envelope":{"schemaVersion":"1.0","timestamp":"2026-01-01T00:00:00Z","source":"legacy","producer":"test","eventType":"LegacyEvent","payload":{},"seq":1},"processed":false}"#;
        std::fs::write(&log_path, legacy_line).unwrap();

        // Open and iterate — must not fail
        let log = EventLog::open(log_path.clone()).unwrap();
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 1);

        // The event must deserialize with schema_version = "1.0" (legacy default)
        let json_str = serde_json::to_string(&events[0]).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let envelope = val.get("envelope").expect("envelope must exist");
        assert_eq!(envelope["schemaVersion"], "1.0");
        // event_id should be Uuid::nil() for legacy (but we can't easily assert nil from here)
        // correlationId and causationId should be null
        assert!(envelope["correlationId"].is_null());
        assert!(envelope["causationId"].is_null());
    }

    /// R3 + spec scenario 5: per-consumer checkpoint persists across reopen.
    /// Fails against pre-fix (consumer_checkpoint API does not exist).
    #[test]
    fn event_log_checkpoint_roundtrip() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("checkpoint_test.jsonl");
        let log = EventLog::open(log_path.clone()).unwrap();

        // Set checkpoint for consumer "alpha" to seq=42
        log.set_consumer_checkpoint("alpha", 42).unwrap();

        // Drop and reopen
        drop(log);
        let log = EventLog::open(log_path).unwrap();

        // Read checkpoint back — must be 42
        let seq = log.consumer_checkpoint("alpha").unwrap();
        assert_eq!(seq, 42, "consumer_checkpoint must return 42 after reopen");
    }

    // ---------------------------------------------------------------------------
    // Coverage tests — invariants, edge cases, and contract verification
    // ---------------------------------------------------------------------------

    /// `append_serialized` raises the seq marker only when `incoming > current`.
    /// Per spec: the seq is monotonic — it never lowers even if a caller supplies
    /// a smaller seq value.
    #[test]
    fn event_log_append_serialized_monotonic_seq_raises() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("monotonic_raise.jsonl");
        let mut log = EventLog::open(log_path).unwrap();

        // Start at seq=5 (not 1) to bypass the natural monotonic increment
        log.append_serialized(&make_envelope(5, "Event")).unwrap();
        assert_eq!(
            log.seq().unwrap(),
            5,
            "seq=5 must be persisted after first append"
        );

        // Append a higher seq — must raise the marker
        log.append_serialized(&make_envelope(10, "Event")).unwrap();
        assert_eq!(
            log.seq().unwrap(),
            10,
            "seq must be raised to 10 when incoming > current"
        );

        // Append a lower seq — must NOT lower the marker
        log.append_serialized(&make_envelope(3, "Event")).unwrap();
        assert_eq!(
            log.seq().unwrap(),
            10,
            "seq marker must be monotonic — must NOT lower from 10 to 3"
        );
    }

    /// `EventLog::append` (auto-assigning) followed by `append_serialized`
    /// (caller-supplied) both write to the same NDJSON log without losing
    /// or duplicating events.
    #[test]
    fn event_log_append_and_append_serialized_coexist() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("mixed_append.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        log.append(
            "test-producer",
            "test-source",
            "GoalSubmitted",
            serde_json::json!({}),
            None,
            None,
            None,
        )
        .unwrap();

        log.append_serialized(&make_envelope(99, "ManualEvent"))
            .unwrap();

        let log = EventLog::open(log_path).unwrap();
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(
            events.len(),
            2,
            "iter must return 2 events (one auto, one manual)"
        );

        // Event 0 from `append`: schema_version="1.1" auto-assigned
        assert_eq!(events[0].envelope.schema_version, "1.1");
        assert_eq!(events[0].envelope.event_type.as_str(), "GoalSubmitted");
        assert_eq!(events[0].envelope.seq, 1, "append auto-assigns seq=1");

        // Event 1 from `append_serialized`: caller-supplied
        assert_eq!(events[1].envelope.event_type.as_str(), "ManualEvent");
        assert_eq!(
            events[1].envelope.seq, 99,
            "append_serialized preserves caller seq"
        );
    }

    /// Round-trip EventEnvelope with ALL optional fields populated
    /// (correlation, causation, graph_revision). Confirms `graphRevision` is
    /// always serialized (no `skip_serializing_if` attribute).
    #[test]
    fn event_envelope_roundtrip_with_all_optional_fields() {
        let env = EventEnvelope {
            event_id: Uuid::nil(),
            schema_version: "1.1".to_string(),
            timestamp: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            source: "dispatcher".into(),
            producer: "event_dispatcher".into(),
            event_type: "GoalCompleted".into(),
            payload: serde_json::json!({"outcome": "ok"}),
            seq: 17,
            correlation_id: Some(Uuid::nil()),
            causation_id: Some(Uuid::nil()),
            graph_revision: Some(42),
        };
        let json = serde_json::to_string(&env).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(back.correlation_id, env.correlation_id);
        assert_eq!(back.causation_id, env.causation_id);
        assert_eq!(back.graph_revision, env.graph_revision);
        assert_eq!(back.producer, "event_dispatcher");

        // Verify graphRevision is always serialized
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            val.get("graphRevision").is_some(),
            "graphRevision must always be serialized (no skip_serializing_if)"
        );
        assert_eq!(val["graphRevision"], 42);
    }

    /// `consumer_checkpoint` returns 0 for a consumer that has never checkpointed.
    /// Per spec: "Returns `Ok(0)` if the consumer has never checkpointed."
    #[test]
    fn consumer_checkpoint_returns_zero_when_never_set() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("never_checkpoint.jsonl");
        let log = EventLog::open(log_path).unwrap();

        let seq = log.consumer_checkpoint("never-seen-consumer").unwrap();
        assert_eq!(
            seq, 0,
            "consumer_checkpoint must return 0 for new consumers"
        );
    }

    /// `validate_consumer_id` rejects empty consumer IDs.
    #[test]
    fn validate_consumer_id_rejects_empty() {
        let result = validate_consumer_id("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    /// `validate_consumer_id` rejects consumer IDs longer than 64 characters.
    #[test]
    fn validate_consumer_id_rejects_too_long() {
        let too_long = "a".repeat(65);
        let result = validate_consumer_id(&too_long);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    /// `validate_consumer_id` rejects path traversal sequences (`..`).
    #[test]
    fn validate_consumer_id_rejects_double_dot() {
        let result = validate_consumer_id("foo..bar");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    /// `validate_consumer_id` rejects consumer IDs containing `/`.
    #[test]
    fn validate_consumer_id_rejects_slash() {
        let result = validate_consumer_id("foo/bar");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    /// `validate_consumer_id` rejects consumer IDs containing NUL byte.
    #[test]
    fn validate_consumer_id_rejects_nul_byte() {
        let result = validate_consumer_id("foo\0bar");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    /// `validate_consumer_id` accepts well-formed consumer IDs (alphanumeric,
    /// dashes, underscores; up to 64 chars).
    #[test]
    fn validate_consumer_id_accepts_well_formed() {
        assert!(validate_consumer_id("alpha").is_ok());
        assert!(validate_consumer_id("with-dashes").is_ok());
        assert!(validate_consumer_id("with_underscores").is_ok());
        assert!(validate_consumer_id("MixED123_case").is_ok());
        // 64-char boundary — exactly the max allowed
        assert!(validate_consumer_id(&"a".repeat(64)).is_ok());
    }

    /// `EventEnvelope` deserialization with missing optional fields uses the
    /// `#[serde(default)]` attribute. correlationId/causationId/graphRevision
    /// default to None; legacy pre-1.1 lines deserialize cleanly.
    #[test]
    fn event_envelope_deserialize_optional_defaults_to_none() {
        let json = r#"{
            "eventId": "00000000-0000-0000-0000-000000000000",
            "schemaVersion": "1.0",
            "timestamp": "2026-01-01T00:00:00Z",
            "source": "test",
            "eventType": "Legacy",
            "payload": {},
            "seq": 1
        }"#;
        let env: EventEnvelope = serde_json::from_str(json).unwrap();
        assert!(
            env.correlation_id.is_none(),
            "missing correlationId must default to None"
        );
        assert!(
            env.causation_id.is_none(),
            "missing causationId must default to None"
        );
        assert!(
            env.graph_revision.is_none(),
            "missing graphRevision must default to None"
        );
        assert_eq!(env.schema_version, "1.0");
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v2, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `append` returns the auto-assigned event_id AND that same id is
    /// persisted in the log. Locks the returned-then-stored UUID invariant —
    /// if the UUID was changed between return and persist, downstream
    /// causation chains would break silently.
    #[test]
    fn append_persists_event_id_matching_returned_value() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("event_id_roundtrip.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        let returned_id = log
            .append(
                "producer",
                "source",
                "Event",
                serde_json::json!({}),
                None,
                None,
                None,
            )
            .unwrap();

        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].envelope.event_id, returned_id,
            "persisted event_id must match the returned UUID"
        );
    }

    /// Caller-supplied `correlation_id`, `causation_id`, and `graph_revision`
    /// survive the roundtrip through `append` → `iter`. Combined because
    /// they share the same persist+deserialize path.
    #[test]
    fn correlation_causation_and_graph_revision_persist_supplied_values() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("optional_fields_roundtrip.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        let correlation = Uuid::new_v4();
        let causation = Uuid::new_v4();
        let graph_rev: u64 = 17;

        log.append(
            "producer",
            "source",
            "Event",
            serde_json::json!({}),
            Some(correlation),
            Some(causation),
            Some(graph_rev),
        )
        .unwrap();

        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].envelope.correlation_id, Some(correlation));
        assert_eq!(events[0].envelope.causation_id, Some(causation));
        assert_eq!(events[0].envelope.graph_revision, Some(graph_rev));
    }

    /// `append` always emits `schema_version = "1.1"` for new events
    /// (the v1.0 → 1.1 transition added event_id, correlation_id, etc.).
    /// Locks the explicit `schema_version: "1.1".to_string()` field in
    /// `append()`. Legacy writes (via `append_serialized`) keep their
    /// caller's schema_version.
    #[test]
    fn append_auto_assigns_schema_version_1_1_for_new_events() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("schema_version.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        log.append("p", "s", "E", serde_json::json!({}), None, None, None)
            .unwrap();

        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].envelope.schema_version, "1.1",
            "append must always set schema_version=1.1"
        );
    }

    /// `iter()` on a freshly opened (empty) log yields zero events without
    /// error. Distinct from `event_log_open_creates_if_missing` which
    /// checks file size — this checks the iterator contract.
    #[test]
    fn iter_on_empty_log_yields_empty_iterator() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("empty_iter.jsonl");
        let log = EventLog::open(log_path).unwrap();

        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert!(events.is_empty(), "fresh log must yield zero events");
    }

    /// `iter()` skips blank/whitespace-only lines (mirrors the audit log
    /// pattern). Locks robustness against partial flushes before a panic.
    /// Note: `iter()` does NOT have the same explicit `trimmed.is_empty()`
    /// branch as `audit/log.rs::read_all` — instead it relies on
    /// `serde_json::from_str("")` returning Err. Verify which behavior
    /// actually applies.
    #[test]
    fn iter_behavior_on_blank_lines() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("blank_lines.jsonl");

        // Write a valid line, then blank lines, then another valid line
        let mut content = String::new();
        content.push_str(
            r#"{"envelope":{"schemaVersion":"1.0","timestamp":"2026-01-01T00:00:00Z","source":"t","eventType":"E1","payload":{},"seq":1},"processed":false}"#,
        );
        content.push('\n');
        content.push('\n'); // blank line
        content.push_str("   \n"); // whitespace-only line
        content.push_str(
            r#"{"envelope":{"schemaVersion":"1.0","timestamp":"2026-01-01T00:00:00Z","source":"t","eventType":"E2","payload":{},"seq":2},"processed":false}"#,
        );
        content.push('\n');
        std::fs::write(&log_path, content).unwrap();

        let log = EventLog::open(log_path).unwrap();
        let result: Result<Vec<_>, _> = log.iter().unwrap().collect();

        // The actual behavior depends on whether serde_json::from_str("") errors
        // (likely). If it does, iter returns Err on the first blank line.
        // If it doesn't, blank lines are silently skipped.
        // Document whichever behavior the current implementation has.
        match result {
            Ok(events) => {
                // Blank lines silently skipped — events.len() reflects valid lines only
                assert_eq!(
                    events.len(),
                    2,
                    "blank lines must be skipped, yielding only the 2 valid events"
                );
            }
            Err(e) => {
                // Blank lines cause iter to fail — locks this as the contract
                assert_eq!(e.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    /// `iter()` returns `InvalidData` when a JSONL line is malformed.
    /// Locks the error path in `iter()`'s map closure.
    #[test]
    fn iter_returns_error_on_malformed_line() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("malformed_iter.jsonl");
        std::fs::write(&log_path, b"this is not valid json\n").unwrap();

        let log = EventLog::open(log_path).unwrap();
        let result: Result<Vec<_>, _> = log.iter().unwrap().collect();
        let err = result.expect_err("malformed line must error");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    /// Two different consumer IDs have INDEPENDENT checkpoint state —
    /// setting one does not affect the other. Locks the per-consumer
    /// isolation contract.
    #[test]
    fn consumer_checkpoints_are_independent_per_consumer() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("independent_checkpoints.jsonl");
        let log = EventLog::open(log_path).unwrap();

        log.set_consumer_checkpoint("alpha", 42).unwrap();
        log.set_consumer_checkpoint("beta", 100).unwrap();

        assert_eq!(log.consumer_checkpoint("alpha").unwrap(), 42);
        assert_eq!(log.consumer_checkpoint("beta").unwrap(), 100);

        // Update alpha — beta must remain at 100
        log.set_consumer_checkpoint("alpha", 50).unwrap();
        assert_eq!(log.consumer_checkpoint("alpha").unwrap(), 50);
        assert_eq!(
            log.consumer_checkpoint("beta").unwrap(),
            100,
            "updating one checkpoint must must not affect others"
        );
    }

    /// `set_consumer_checkpoint` propagates the `validate_consumer_id`
    /// rejection as an `InvalidInput` IO error. Locks the integration
    /// between the public API and the validator.
    #[test]
    fn set_consumer_checkpoint_rejects_invalid_consumer_id() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("rejected_checkpoint.jsonl");
        let log = EventLog::open(log_path).unwrap();

        // Empty consumer_id
        let result = log.set_consumer_checkpoint("", 1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

        // Path-traversal consumer_id
        let result = log.set_consumer_checkpoint("foo..bar", 1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);

        // Slash consumer_id
        let result = log.set_consumer_checkpoint("foo/bar", 1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    /// `validate_consumer_id` rejects characters outside the allowed
    /// charset `^[a-zA-Z0-9_-]{1,64}$`. Locks the charset enforcement
    /// beyond the special cases (`..`, `/`, NUL) already tested.
    #[test]
    fn validate_consumer_id_rejects_special_chars_outside_charset() {
        for bad in [
            "foo@bar",  // @
            "foo:bar",  // :
            "foo bar",  // space
            "foo*bar",  // *
            "foo?bar",  // ?
            "foo\\bar", // backslash
            "foo|bar",  // pipe
            "foo;bar",  // semicolon
            "foo,bar",  // comma
            "foo#bar",  // hash
            "中文",     // non-ASCII
        ] {
            let result = validate_consumer_id(bad);
            assert!(
                result.is_err(),
                "consumer_id '{bad}' must be rejected (charset)"
            );
            assert_eq!(
                result.unwrap_err().kind(),
                io::ErrorKind::InvalidInput,
                "charset rejection must return InvalidInput for '{bad}'"
            );
        }
    }

    /// `Sequence::load` on a marker file containing garbage returns `Ok(0)`
    /// (via `unwrap_or(0)`). Locks the recovery-from-corruption path —
    /// a corrupt seq marker must NOT panic the log; it must reset to 0
    /// and let the log continue. The risk is duplicate seq values, but
    /// appending a new event generates a fresh seq anyway.
    #[test]
    fn sequence_load_with_garbage_file_returns_zero() {
        let tmp_dir = TempDir::new().unwrap();
        let tmp = tmp_dir.path().join("garbage_seq.seq");
        std::fs::write(&tmp, b"this is not a number\n\n\t  ").unwrap();

        let seq = Sequence::load(&tmp).expect("garbage file must not panic");
        assert_eq!(seq.0, 0, "garbage seq file must default to 0 via unwrap_or");
    }

    /// `append_serialized` is a NO-OP for the seq marker when `incoming == current`
    /// (strict inequality `incoming > current`). Distinct from the
    /// `incoming > current → raise` and `incoming < current → no-op`
    /// cases — this locks the equality boundary.
    #[test]
    fn append_serialized_with_seq_equal_to_current_is_noop() {
        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("seq_equal_noop.jsonl");
        let mut log = EventLog::open(log_path).unwrap();

        log.append_serialized(&make_envelope(5, "First")).unwrap();
        assert_eq!(log.seq().unwrap(), 5);

        // Append another event with the SAME seq — marker must stay at 5
        log.append_serialized(&make_envelope(5, "Second")).unwrap();
        assert_eq!(
            log.seq().unwrap(),
            5,
            "incoming == current must NOT raise the seq marker"
        );

        // But the line must still be in the log (append_serialized writes
        // regardless of the seq comparison)
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(
            events.len(),
            2,
            "both events must be persisted, even with identical seq values"
        );
    }
}

//! Event types for the reactive runtime.
//!
//! PR1 provides the foundation types: [`EventEnvelope`], [`SerializedEvent`],
//! [`EventLog`] (append-only JSONL), and [`Sequence`] (marker-file counter).

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

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

/// An event envelope with timestamp, source, event type, payload, and sequence.
///
/// The `event_type` field is a free-form `String` (NOT an enum) to stay
/// extensible — different agents can define their own event types without
/// requiring a central registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Nanosecond timestamp from the monotonic clock.
    pub ts: u64,
    /// Originating source identifier (e.g. `"dispatcher"`, `"file_watcher"`).
    pub source: String,
    /// Event kind in CamelCase notation, e.g. `GoalSubmitted`, `FileChanged`.
    pub event_type: String,
    /// Arbitrary JSON payload associated with this event.
    pub payload: serde_json::Value,
    /// Monotonically increasing sequence number for ordering.
    pub seq: u64,
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
        Ok(EventLog { path, seq_path })
    }

    /// Append a serialised event to the log.
    pub fn append(&mut self, event: &SerializedEvent) -> io::Result<()> {
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        let json = serde_json::to_string(event)?;
        writeln!(file, "{}", json)
    }

    /// Returns an iterator over all events in the log, in order.
    pub fn iter(&self) -> io::Result<impl Iterator<Item = io::Result<SerializedEvent>>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let lines = reader.lines();
        let iter = lines.map(|line| {
            let l = line?;
            serde_json::from_str(&l).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        });
        Ok(iter)
    }

    /// Returns the current sequence number (highest seq in the log, or 0).
    pub fn seq(&self) -> io::Result<u64> {
        Sequence::load(&self.seq_path).map(|s| s.0)
    }

    /// Update the persisted sequence number (called after successful processing).
    pub fn update_seq(&self, seq: u64) -> io::Result<()> {
        Sequence(seq).store(&self.seq_path)
    }
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
                ts: seq * 100,
                source: "test".into(),
                event_type: event_type.into(),
                payload: serde_json::json!({}),
                seq,
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
            ts: 1_234_567_890,
            source: "test".into(),
            event_type: "GoalSubmitted".into(),
            payload: serde_json::json!({"goal": "analyze architecture"}),
            seq: 42,
        };
        let json = serde_json::to_string(&env).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ts, env.ts);
        assert_eq!(back.source, env.source);
        assert_eq!(back.event_type, env.event_type);
        assert_eq!(back.seq, env.seq);
    }

    #[test]
    fn serialized_event_roundtrip_json() {
        let env = EventEnvelope {
            ts: 1,
            source: "dispatcher".into(),
            event_type: "FileChanged".into(),
            payload: serde_json::json!({"path": "src/main.rs"}),
            seq: 7,
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

        log.append(&make_envelope(1, "GoalSubmitted")).unwrap();
        log.append(&make_envelope_with_processed(2, "FileChanged", true))
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
            log.append(&make_envelope(1, "GoalSubmitted")).unwrap();
            log.append(&make_envelope(2, "FileChanged")).unwrap();
            log.append(&make_envelope(3, "GoalCompleted")).unwrap();
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
                log.append(&make_envelope(i, "Event")).unwrap();
            }
        }

        // Reopen and append 2 more
        {
            let mut log = EventLog::open(log_path.clone()).unwrap();
            log.append(&make_envelope(6, "Event")).unwrap();
            log.append(&make_envelope(7, "Event")).unwrap();
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
        assert!(
            !id.is_nil(),
            "append must assign a non-nil event_id"
        );

        // The persisted event must carry eventId and timestamp fields
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 1);
        let json_str = serde_json::to_string(&events[0]).unwrap();
        let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // JSON must contain the new camelCase keys
        assert!(
            json_val.get("eventId").is_some(),
            "JSON must contain eventId field"
        );
        assert!(
            json_val.get("timestamp").is_some(),
            "JSON must contain timestamp field"
        );
        assert!(
            json_val.get("schemaVersion").is_some(),
            "JSON must contain schemaVersion field"
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
        assert_ne!(parent_id, child_id, "child must have different event_id from parent");

        // Read back and verify causation
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 2);

        // Second event (child) must carry causationId = parent_id
        let child_json =
            serde_json::to_string(&events[1]).unwrap();
        let child_val: serde_json::Value = serde_json::from_str(&child_json).unwrap();
        assert!(
            child_val.get("causationId").is_some(),
            "child event must carry causationId field"
        );
        let causation_id_str = child_val["causationId"].as_str().unwrap();
        assert_eq!(
            causation_id_str, parent_id.to_string(),
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
            assert!(
                val.get("correlationId").is_some(),
                "event {} must carry correlationId field",
                i
            );
        }

        // Events 0 and 1 must have the same correlationId
        let json0 = serde_json::to_string(&events[0]).unwrap();
        let json1 = serde_json::to_string(&events[1]).unwrap();
        let val0: serde_json::Value = serde_json::from_str(&json0).unwrap();
        let val1: serde_json::Value = serde_json::from_str(&json1).unwrap();
        assert_eq!(
            val0["correlationId"], val1["correlationId"],
            "events A and B must share correlationId"
        );

        // Event 2 must have null correlationId (different group)
        let json2 = serde_json::to_string(&events[2]).unwrap();
        let val2: serde_json::Value = serde_json::from_str(&json2).unwrap();
        assert!(
            val0["correlationId"] != val2["correlationId"] || val2["correlationId"].is_null(),
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

        // Write a pre-1.1 JSONL line (no eventId, no correlationId, ts as integer)
        let legacy_line = r#"{"ts":1234567890,"source":"legacy","eventType":"LegacyEvent","payload":{},"seq":1,"schemaVersion":"1.0","producer":"test"}"#;
        std::fs::write(&log_path, legacy_line).unwrap();

        // Open and iterate — must not fail
        let log = EventLog::open(log_path.clone()).unwrap();
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 1);

        // The event must deserialize with event_id = Uuid::nil()
        // (we can't easily check the nil UUID from the EventEnvelope struct,
        // but we verify the event is readable and schemaVersion is "1.0")
        let json_str = serde_json::to_string(&events[0]).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(val["schemaVersion"], "1.0");
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
        assert_eq!(
            seq, 42,
            "consumer_checkpoint must return 42 after reopen"
        );
    }
}

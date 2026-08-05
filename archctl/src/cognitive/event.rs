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
    /// The sequence marker file is placed alongside the log at `event.log.seq`.
    pub fn open(path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Touch the file so it exists even if no events are written.
        File::create(&path)?;
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

    #[test]
    fn sequence_increment_and_persist() {
        let tmp_dir = std::env::temp_dir();
        let tmp = tmp_dir.join("archctl_seq_test");
        let mut seq = Sequence(0);
        assert_eq!(seq.next(), 1);
        seq.store(&tmp).unwrap();
        let loaded = Sequence::load(&tmp).unwrap();
        assert_eq!(loaded.0, 1);
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn sequence_load_missing_file_returns_zero() {
        let tmp = std::env::temp_dir().join("archctl_nonexistent_seq_file");
        let seq = Sequence::load(&tmp).unwrap();
        assert_eq!(seq.0, 0);
        let _ = fs::remove_file(&tmp);
    }

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

    #[test]
    fn event_log_append_and_iter() {
        let tmp_dir = std::env::temp_dir();
        let log_path = tmp_dir.join("archctl_event_log_test.jsonl");
        let mut log = EventLog::open(log_path.clone()).unwrap();

        let env1 = EventEnvelope {
            ts: 100,
            source: "test".into(),
            event_type: "GoalSubmitted".into(),
            payload: serde_json::json!({}),
            seq: 1,
        };
        log.append(&SerializedEvent {
            envelope: env1,
            processed: false,
        })
        .unwrap();

        let env2 = EventEnvelope {
            ts: 200,
            source: "test".into(),
            event_type: "FileChanged".into(),
            payload: serde_json::json!({}),
            seq: 2,
        };
        log.append(&SerializedEvent {
            envelope: env2,
            processed: true,
        })
        .unwrap();

        let iter = log.iter().unwrap();
        let events: Vec<_> = iter.collect::<io::Result<Vec<_>>>().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].envelope.seq, 1);
        assert_eq!(events[1].envelope.seq, 2);
        assert!(!events[0].processed);
        assert!(events[1].processed);

        // Cleanup
        let _ = fs::remove_file(&log_path);
        let _ = fs::remove_file(log_path.with_extension("seq"));
    }

    #[test]
    fn event_log_seq_persistence() {
        let tmp_dir = std::env::temp_dir();
        let log_path = tmp_dir.join("archctl_event_seq_test.jsonl");
        let log = EventLog::open(log_path.clone()).unwrap();

        log.update_seq(99).unwrap();
        let loaded_seq = log.seq().unwrap();
        assert_eq!(loaded_seq, 99);

        // Cleanup
        let _ = fs::remove_file(&log_path);
        let _ = fs::remove_file(log_path.with_extension("seq"));
    }
}

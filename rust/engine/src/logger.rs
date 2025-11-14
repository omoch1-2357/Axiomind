use serde::{Deserialize, Serialize};

use crate::cards::Card;
use crate::player::PlayerAction;

/// Represents a betting street in Texas Hold'em poker.
/// Defines the four stages of a poker hand.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Street {
    /// Before flop (hole cards dealt)
    Preflop,
    /// After flop (3 community cards)
    Flop,
    /// After turn (4th community card)
    Turn,
    /// After river (5th community card)
    River,
}

/// Records a single player action during a hand.
/// Associates the action with the player and the street when it occurred.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    /// Player identifier (0 or 1)
    pub player_id: usize,
    /// The betting street when this action occurred
    pub street: Street,
    /// The action taken by the player
    pub action: PlayerAction,
}

/// Complete record of a poker hand including all actions, board cards, and outcome.
/// Serialized to JSONL format for hand history storage and replay.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HandRecord {
    /// Unique identifier for this hand (format: YYYYMMDD-NNNNNN)
    pub hand_id: String,
    /// RNG seed used for deck shuffling (enables deterministic replay)
    pub seed: Option<u64>,
    /// Chronological list of all player actions
    pub actions: Vec<ActionRecord>,
    /// Community cards on the board (up to 5 cards)
    pub board: Vec<Card>,
    /// Hand result summary (winner, pot size, etc.)
    pub result: Option<String>,
    /// Timestamp when the hand was played (RFC3339 format)
    #[serde(default)]
    pub ts: Option<String>,
    /// Additional metadata (extensible JSON object)
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
    /// Showdown information if hand went to showdown
    #[serde(default)]
    pub showdown: Option<ShowdownInfo>,
}

pub fn format_hand_id(yyyymmdd: &str, seq: u32) -> String {
    format!("{}-{:06}", yyyymmdd, seq)
}

use chrono::{SecondsFormat, Utc};
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct HandLogger {
    writer: Option<BufWriter<File>>,
    date: String,
    seq: u32,
}

/// Information about the showdown phase when hands are revealed.
/// Records which players won and any relevant notes about the outcome.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ShowdownInfo {
    /// List of player IDs who won the hand
    pub winners: Vec<usize>,
    /// Optional notes about the showdown (e.g., "split pot", "flush over straight")
    #[serde(default)]
    pub notes: Option<String>,
}

impl HandLogger {
    pub fn create<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                let _ = create_dir_all(parent);
            }
        }
        let f = File::create(path)?;
        Ok(Self {
            writer: Some(BufWriter::new(f)),
            date: "19700101".to_string(),
            seq: 0,
        })
    }

    pub fn with_seq_for_test(date: &str) -> Self {
        Self {
            writer: None,
            date: date.to_string(),
            seq: 0,
        }
    }

    pub fn next_id(&mut self) -> String {
        self.seq += 1;
        format_hand_id(&self.date, self.seq)
    }

    pub fn write(&mut self, record: &HandRecord) -> std::io::Result<()> {
        // inject timestamp if missing
        let mut rec = record.clone();
        if rec.ts.is_none() {
            rec.ts = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
        }
        let line = serde_json::to_string(&rec).map_err(std::io::Error::other)?;
        if let Some(w) = &mut self.writer {
            w.write_all(line.as_bytes())?;
            w.write_all(b"\n")?;
            w.flush()?;
        }
        Ok(())
    }
}

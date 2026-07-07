// Copyright (C) 2026 AutoResearch Chain contributors
//
// This file is part of AutoResearch Chain.
//
// AutoResearch Chain is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// AutoResearch Chain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Single-sequencer ordering log (Milestone E2).
//!
//! **This is deliberately boring, explicitly temporary scaffolding.** The
//! useful-work game does not order the chain; a single authority does,
//! and every follower can verify everything about the ordering except
//! censorship: entries are hash-chained, authority-signed, and each
//! embedded transaction still carries its own actor signature (verified
//! on replay). The trust model is stated, not hidden — replacing this
//! with permissioned-set rotation or bonded ordering later does not
//! change the transaction or state model.

use serde::{Deserialize, Serialize};

/// One ordered transaction in the log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// Sequence number, starting at 0.
    pub seq: u64,
    /// Hex hash of the previous entry ("00"*32 for the first).
    pub prev_hash: String,
    /// Transaction kind (CLI command name).
    pub kind: String,
    /// Transaction payload (including any actor signature).
    pub payload: serde_json::Value,
    /// Hex hash of this entry (over seq|prev_hash|kind|payload).
    pub entry_hash: String,
    /// Authority Ed25519 signature over the ordering message.
    pub authority_sig: String,
}

/// Hash of the entry content (before signing).
pub fn entry_hash(seq: u64, prev_hash: &str, kind: &str, payload: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(&serde_json::json!({
        "seq": seq,
        "prev_hash": prev_hash,
        "kind": kind,
        "payload": payload,
    }))
    .expect("entry serialization failed");
    blake3::hash(canonical.as_bytes()).to_hex().to_string()
}

/// The message the ordering authority signs for an entry.
pub fn ordering_message(seq: u64, entry_hash: &str) -> Vec<u8> {
    format!("arc-ordering-v1|{seq}|{entry_hash}").into_bytes()
}

pub const GENESIS_PREV: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Append-only JSONL log persisted next to the state file.
pub struct OrderingLog {
    pub entries: Vec<LogEntry>,
    path: std::path::PathBuf,
}

impl OrderingLog {
    /// Open (or create) the log at `path`, loading existing entries.
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let mut entries = Vec::new();
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read log {}: {}", path.display(), e))?;
            for (i, line) in content.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let entry: LogEntry = serde_json::from_str(line)
                    .map_err(|e| format!("corrupt log line {}: {}", i + 1, e))?;
                entries.push(entry);
            }
        }
        Ok(Self { entries, path: path.to_path_buf() })
    }

    pub fn tip_hash(&self) -> String {
        self.entries
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_else(|| GENESIS_PREV.to_string())
    }

    pub fn next_seq(&self) -> u64 {
        self.entries.len() as u64
    }

    /// Build, sign, append, and persist a new entry.
    pub fn append(
        &mut self,
        kind: &str,
        payload: serde_json::Value,
        authority: &arc_identity::Keypair,
    ) -> Result<&LogEntry, String> {
        let seq = self.next_seq();
        let prev_hash = self.tip_hash();
        let hash = entry_hash(seq, &prev_hash, kind, &payload);
        let sig = authority.sign(&ordering_message(seq, &hash));
        let entry = LogEntry {
            seq,
            prev_hash,
            kind: kind.to_string(),
            payload,
            entry_hash: hash,
            authority_sig: sig.iter().map(|b| format!("{:02x}", b)).collect(),
        };
        let line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("cannot open log {}: {}", self.path.display(), e))?;
        writeln!(file, "{}", line).map_err(|e| e.to_string())?;
        self.entries.push(entry);
        Ok(self.entries.last().unwrap())
    }
}

/// Verify one entry against the expected sequence position, chain tip,
/// and ordering authority public key.
pub fn verify_entry(
    entry: &LogEntry,
    expected_seq: u64,
    expected_prev: &str,
    authority_pk: &[u8; 32],
) -> Result<(), String> {
    if entry.seq != expected_seq {
        return Err(format!(
            "log gap: expected seq {}, got {}",
            expected_seq, entry.seq
        ));
    }
    if entry.prev_hash != expected_prev {
        return Err(format!(
            "chain break at seq {}: expected prev {}, got {}",
            entry.seq, expected_prev, entry.prev_hash
        ));
    }
    let hash = entry_hash(entry.seq, &entry.prev_hash, &entry.kind, &entry.payload);
    if hash != entry.entry_hash {
        return Err(format!("entry {} hash mismatch", entry.seq));
    }
    let sig_bytes: Vec<u8> = (0..entry.authority_sig.len() / 2)
        .map(|i| u8::from_str_radix(&entry.authority_sig[i * 2..i * 2 + 2], 16))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("entry {} signature hex: {}", entry.seq, e))?;
    arc_identity::verify(authority_pk, &ordering_message(entry.seq, &hash), &sig_bytes)
        .map_err(|e| format!("entry {} authority signature: {}", entry.seq, e))
}

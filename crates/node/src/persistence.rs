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

//! Minimal whole-state snapshot persistence.
//!
//! Saves and loads the complete `SimulatorState` as a JSON file.
//! This is the narrowest honest persistence mechanism for a local
//! single-node runtime. It does not attempt granular object storage,
//! incremental updates, or concurrent access.
//!
//! # Format
//!
//! The state file is a single JSON document containing the serialized
//! `SimulatorState`. It is human-readable and inspectable.
//!
//! # Future
//!
//! This will be replaced by a proper persistence layer (likely SQLite
//! or a custom store) when the node needs incremental persistence,
//! concurrent access, or large state handling. For now, whole-state
//! snapshot is sufficient for local single-node operation.

use std::fs;
use std::io;
use std::path::Path;

use arc_simulator::state::SimulatorState;

/// Save simulator state to a JSON file.
///
/// Overwrites the file if it already exists. Creates parent
/// directories if they don't exist.
pub fn save_state(state: &SimulatorState, path: &Path) -> Result<(), PersistenceError> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(PersistenceError::Io)?;
        }
    }

    let json = serde_json::to_string_pretty(state).map_err(PersistenceError::Serialize)?;
    fs::write(path, json).map_err(PersistenceError::Io)?;

    Ok(())
}

/// Load simulator state from a JSON file.
pub fn load_state(path: &Path) -> Result<SimulatorState, PersistenceError> {
    let contents = fs::read_to_string(path).map_err(PersistenceError::Io)?;
    let state = serde_json::from_str(&contents).map_err(PersistenceError::Deserialize)?;
    Ok(state)
}

/// Errors from state persistence operations.
#[derive(Debug)]
pub enum PersistenceError {
    /// I/O error (file not found, permission denied, etc.).
    Io(io::Error),
    /// Serialization failed.
    Serialize(serde_json::Error),
    /// Deserialization failed (corrupt or incompatible state file).
    Deserialize(serde_json::Error),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Serialize(e) => write!(f, "serialization error: {}", e),
            Self::Deserialize(e) => write!(f, "deserialization error: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("arc_node_test_{}", name));
        path
    }

    #[test]
    fn save_and_load_empty_state() {
        let path = temp_path("empty_state.json");
        let state = SimulatorState::new();

        save_state(&state, &path).unwrap();
        let loaded = load_state(&path).unwrap();

        assert_eq!(loaded.current_epoch, state.current_epoch);
        assert!(loaded.blocks.is_empty());

        // Cleanup.
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_nonexistent_file_returns_error() {
        let path = temp_path("nonexistent_file.json");
        let result = load_state(&path);
        assert!(result.is_err());
    }

    #[test]
    fn save_creates_parent_directories() {
        let mut path = std::env::temp_dir();
        path.push("arc_node_test_nested");
        path.push("subdir");
        path.push("state.json");

        let state = SimulatorState::new();
        save_state(&state, &path).unwrap();

        assert!(path.exists());

        // Cleanup.
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }
}

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

//! Minimal local runtime for AutoResearch Chain.
//!
//! This binary wraps the protocol core into a single-node local runtime.
//! It is a Phase 1 target — the protocol core must be validated in the
//! simulator first.
//!
//! Current capabilities (Phase 1 bootstrap):
//! - `init`: create a fresh protocol state file
//! - `inspect`: display basic info about a saved state
//!
//! Future responsibilities:
//! - Transaction submission flow (genesis, blocks, attestations, challenges)
//! - Local state queries
//! - Event log / state-transition trace
//!
//! Networking is explicitly not the first priority. The node initially
//! runs as a local-only process.

mod persistence;

use std::path::PathBuf;

use arc_simulator::state::SimulatorState;

const DEFAULT_STATE_FILE: &str = "arc-state.json";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("init") => cmd_init(&args[2..]),
        Some("inspect") => cmd_inspect(&args[2..]),
        Some("help") | Some("--help") | Some("-h") => print_usage(),
        _ => print_usage(),
    }
}

fn print_usage() {
    eprintln!("arc-node: minimal local runtime for AutoResearch Chain");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  arc-node init [PATH]       Create a fresh protocol state file");
    eprintln!("  arc-node inspect [PATH]    Display basic info about a saved state");
    eprintln!("  arc-node help              Show this message");
    eprintln!();
    eprintln!("Default state file: {}", DEFAULT_STATE_FILE);
    eprintln!();
    eprintln!("This is a Phase 1 bootstrap. Full transaction submission,");
    eprintln!("queries, and runtime loop are not yet implemented.");
}

fn state_path(args: &[String]) -> PathBuf {
    args.first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STATE_FILE))
}

fn cmd_init(args: &[String]) {
    let path = state_path(args);

    if path.exists() {
        eprintln!("error: state file already exists: {}", path.display());
        eprintln!("Remove it first if you want to reinitialize.");
        std::process::exit(1);
    }

    let state = SimulatorState::new();
    match persistence::save_state(&state, &path) {
        Ok(()) => {
            eprintln!("Created fresh state: {}", path.display());
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_inspect(args: &[String]) {
    let path = state_path(args);

    let state = match persistence::load_state(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    eprintln!("State file: {}", path.display());
    eprintln!("  Epoch:              {}", state.current_epoch);
    eprintln!("  Active domains:     {}", state.domain_registry.domains.len());
    eprintln!("  Blocks:             {}", state.blocks.len());
    eprintln!("  Challenges:         {}", state.challenges.len());
    eprintln!("  Escrow records:     {}", state.escrow_records.len());
    eprintln!("  Pending activations:{}", state.pending_activations.len());
}

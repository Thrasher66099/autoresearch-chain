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
//! Future responsibilities:
//! - Single-node chain state persistence
//! - Transaction submission flow (genesis, blocks, attestations, challenges)
//! - Local state queries
//! - Event log / state-transition trace
//!
//! Networking is explicitly not the first priority. The node initially
//! runs as a local-only process.
//!
//! # Implementation status
//!
//! Not yet implemented. Depends on Phase 0 protocol core validation.

fn main() {
    eprintln!("arc-node: not yet implemented");
    eprintln!("The protocol core must be validated in the simulator first.");
    eprintln!("See docs/implementation-plan.md for build phases.");
    std::process::exit(1);
}

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

//! Local protocol simulator for AutoResearch Chain.
//!
//! # Phase 0.2 implementation
//!
//! The simulator composes the domain-engine, protocol-rules, fork-engine,
//! and challenge-engine into a single local state machine that can execute
//! protocol scenarios deterministically.
//!
//! This is the primary Phase 0 deliverable: make the protocol behave
//! locally, deterministically, and testably.

pub mod state;

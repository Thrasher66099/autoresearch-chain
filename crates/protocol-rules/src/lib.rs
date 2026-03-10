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

//! Deterministic state transition logic for AutoResearch Chain.
//!
//! This crate defines the rules by which the protocol state evolves.
//! It operates on types from `arc-protocol-types` and produces new states
//! or errors — never side effects.
//!
//! All transitions must be deterministic: given the same prior state and
//! the same input, every node must produce the same result.
//!
//! # Scope
//!
//! - Block submission validation
//! - Attestation aggregation rules
//! - Challenge opening and resolution rules
//! - Fork activation and dominance transitions
//! - Frontier settlement
//! - Reward staging and slashing outcomes
//! - Cross-domain integration rules
//!
//! Domain-specific lifecycle logic (genesis, track activation) lives in
//! `arc-domain-engine`. Fork-specific logic lives in `arc-fork-engine`.
//! This crate provides the top-level transition dispatch and coordination.
//!
//! # Implementation status
//!
//! Not yet implemented. The transition API shape is not yet decided.

// TODO: Define a StateTransition trait or equivalent dispatch model.
// TODO: Define protocol error types for invalid transitions.
// TODO: Decide whether transitions operate on a full state snapshot
//       or on a more granular state accessor trait.

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

//! Challenge lifecycle, resolution rules, and remedies
//! for AutoResearch Chain.
//!
//! Challenges are bonded adversarial disputes. Any participant can challenge
//! a block, attestation, attribution claim, or fork dominance declaration
//! by posting a bond and evidence.
//!
//! This crate manages:
//!
//! - Challenge creation and bond handling
//! - Challenge types: block replay, attestation fraud, attribution,
//!   dominance, metric adequacy
//! - Challenge resolution state machine
//! - Remedy application (slashing, reward redistribution, block invalidation)
//! - Challenge window tracking
//! - Escalation rules
//!
//! # Implementation status
//!
//! Not yet implemented.

// TODO: Define challenge state machine (opened → evidence → resolved).
// TODO: Implement resolution logic per challenge type.
// TODO: Implement remedy application.
// TODO: Decide on challenge escalation encoding for v0.

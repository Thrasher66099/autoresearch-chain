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

//! Artifact references, content-addressed metadata, and materialized state
//! storage for AutoResearch Chain.
//!
//! The protocol stores references and commitments on-chain, not large
//! artifacts directly. This crate defines how the protocol references:
//!
//! - Evidence bundles (diffs, configs, logs, metric outputs)
//! - Materialized code snapshots
//! - Dataset references
//! - Evaluation harness references
//! - Environment manifests
//! - Dependency manifests
//!
//! It also manages:
//!
//! - Content-addressed reference resolution
//! - MaterializedState generation triggers (dominance, depth threshold,
//!   scheduled checkpoint, domain policy)
//! - CanonicalFrontierState assembly
//! - CodebaseStateRef resolution (latest frontier vs. historical state)
//! - Data availability verification (can the evidence be fetched?)
//!
//! # Implementation status
//!
//! Not yet implemented. The reference and hash model is an early
//! technical decision that should be locked before extensive use.

// TODO: Define content-addressed reference types.
// TODO: Define artifact metadata schema.
// TODO: Implement materialization trigger logic.
// TODO: Implement frontier state assembly.
// TODO: Decide on data availability checking model.

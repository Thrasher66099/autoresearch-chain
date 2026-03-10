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

//! Fork families, dominance evaluation, and frontier selection
//! for AutoResearch Chain.
//!
//! Forks are a first-class protocol feature, not failure states.
//! Multiple valid improvements targeting the same parent create competing
//! branches. This crate manages:
//!
//! - Fork family identification (sibling branches sharing a common ancestor)
//! - Branch tracking within a domain
//! - Dominance evaluation (which fork is recognized as superior)
//! - Frontier selection (the leading edge of a branch or domain)
//! - Canonical frontier settlement
//! - Cross-fork porting tracking
//! - Fork proliferation detection (for adversarial scenarios)
//!
//! # Implementation status
//!
//! Not yet implemented.

// TODO: Define fork family data structures and ancestor resolution.
// TODO: Implement dominance evaluation criteria.
// TODO: Implement frontier selection algorithm.
// TODO: Implement cross-fork porting detection and attribution.

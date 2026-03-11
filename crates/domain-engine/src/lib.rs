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

//! Domain lifecycle, research track standards, genesis activation,
//! and track tree management for AutoResearch Chain.
//!
//! # Phase 0.2 implementation
//!
//! This implements the first local, deterministic version of:
//! - RTS-1 conformance checking
//! - Genesis activation state machine
//! - TrackTree construction from activated genesis
//! - ProblemDomain + DomainSpec instantiation
//! - Domain registry

pub mod config;
pub mod error;
pub mod genesis;
pub mod registry;
pub mod rts;

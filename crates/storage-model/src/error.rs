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

//! Error types for artifact store operations.

use std::fmt;

use arc_protocol_types::ArtifactHash;

/// Errors from artifact store operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    /// The requested artifact was not found in the store.
    NotFound {
        /// The hash that was looked up.
        hash: ArtifactHash,
    },
    /// A store-internal error (I/O failure, corruption, etc.).
    ///
    /// The in-memory store never produces this variant; it exists for
    /// future persistent implementations.
    Internal {
        /// Human-readable description of the failure.
        message: String,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound { hash } => write!(f, "artifact not found: {}", hash),
            StoreError::Internal { message } => write!(f, "store error: {}", message),
        }
    }
}

impl std::error::Error for StoreError {}

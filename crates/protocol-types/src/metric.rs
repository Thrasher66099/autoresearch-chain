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

//! Protocol metric value abstraction.
//!
//! [`MetricValue`] wraps the internal numeric representation used for
//! settlement-relevant metric quantities (scores, deltas, tolerances).
//! The rest of the protocol operates through this abstraction rather
//! than depending directly on `f64`.
//!
//! # Current representation
//!
//! The internal representation is `f64`. This will be replaced with a
//! deterministic fixed-point or rational type before production use.
//! The wrapper exists now so that the migration surface is contained.

use serde::{Deserialize, Serialize};

/// A metric quantity used in protocol settlement.
///
/// Wraps the protocol's internal numeric representation for scores,
/// deltas, and tolerances. All protocol-settlement-relevant numeric
/// comparisons should go through this type.
///
/// # Determinism note
///
/// The current internal representation is `f64`, which is not
/// deterministic across platforms. `MetricValue` derives `PartialEq`
/// but not `Eq` for this reason. A future phase will replace the
/// internal representation with a deterministic numeric type.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricValue(f64);

impl MetricValue {
    /// Create a new metric value.
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    /// Access the underlying `f64` value.
    ///
    /// Prefer operating through `MetricValue` methods where possible.
    /// This accessor exists for interop with external systems and
    /// display formatting.
    pub fn as_f64(self) -> f64 {
        self.0
    }

    /// Returns `true` if the value is finite (not NaN or infinite).
    pub fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl std::fmt::Display for MetricValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

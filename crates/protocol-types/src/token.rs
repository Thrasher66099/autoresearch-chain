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

//! Protocol token amount abstraction.
//!
//! [`TokenAmount`] wraps `u64` to distinguish economic quantities (fees,
//! bonds, escrow amounts) from other integer fields in the protocol.

use serde::{Deserialize, Serialize};

/// An amount of the protocol's native token denomination.
///
/// Wraps `u64` to prevent accidental confusion between economic
/// quantities and other integer fields (timestamps, budgets, quorum
/// counts). The protocol does not define fractional token amounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TokenAmount(u64);

impl TokenAmount {
    /// Zero token amount.
    pub const ZERO: Self = Self(0);

    /// Create a new token amount.
    pub fn new(amount: u64) -> Self {
        Self(amount)
    }

    /// Access the underlying `u64` value.
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Returns `true` if the amount is zero.
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl std::fmt::Display for TokenAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

//! Strongly typed identifier and reference wrappers.
//!
//! All protocol identifiers use newtype wrappers over `[u8; 32]` to prevent
//! accidental misuse across type boundaries. A [`ValidatorId`] cannot be passed
//! where a [`ProposerId`] is expected, even though both are 32-byte values.
//!
//! [`EpochId`] wraps a `u64` since epochs are sequential, not content-addressed.

use serde::{Deserialize, Serialize};

/// Defines a newtype identifier wrapping `[u8; 32]`.
///
/// Each generated type gets: `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`,
/// custom hex-based `Serialize`/`Deserialize`, a `ZERO` constant,
/// `from_bytes`, `as_bytes`, and `Debug`/`Display` implementations
/// showing a hex prefix.
///
/// Serialization uses lowercase hex strings (64 chars for 32 bytes).
/// This allows these types to work as JSON map keys and produces
/// human-readable output.
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub [u8; 32]);

        impl $name {
            /// The all-zero identifier. Useful as a placeholder in tests.
            pub const ZERO: Self = Self([0u8; 32]);

            /// Create from a raw 32-byte array.
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// View the underlying bytes.
            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            /// Encode as a lowercase hex string.
            fn to_hex(&self) -> String {
                let mut s = String::with_capacity(64);
                for byte in &self.0 {
                    use std::fmt::Write;
                    write!(s, "{:02x}", byte).unwrap();
                }
                s
            }

            /// Decode from a hex string.
            fn from_hex(s: &str) -> Result<Self, String> {
                if s.len() != 64 {
                    return Err(format!(
                        "expected 64 hex chars for {}, got {}",
                        stringify!($name),
                        s.len()
                    ));
                }
                let mut bytes = [0u8; 32];
                for i in 0..32 {
                    bytes[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                        .map_err(|e| format!("invalid hex in {}: {}", stringify!($name), e))?;
                }
                Ok(Self(bytes))
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(", stringify!($name))?;
                for byte in &self.0[..4] {
                    write!(f, "{:02x}", byte)?;
                }
                write!(f, "\u{2026})")
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for byte in &self.0[..4] {
                    write!(f, "{:02x}", byte)?;
                }
                write!(f, "\u{2026}")
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.to_hex())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::from_hex(&s).map_err(serde::de::Error::custom)
            }
        }
    };
}

define_id! {
    /// Unique identifier for a problem domain.
    DomainId
}

define_id! {
    /// Unique identifier for a block within the protocol.
    BlockId
}

define_id! {
    /// Unique identifier for a genesis block.
    ///
    /// Also serves as the root reference for a track tree, since each track
    /// tree is rooted at exactly one genesis block.
    GenesisBlockId
}

define_id! {
    /// Unique identifier for a fork family within a domain.
    ForkFamilyId
}

define_id! {
    /// Unique identifier for a track tree (domain-scoped descendant tree).
    TrackTreeId
}

define_id! {
    /// Unique identifier for a challenge record.
    ChallengeId
}

define_id! {
    /// Unique identifier for a materialized state snapshot.
    MaterializedStateId
}

define_id! {
    /// Content-addressed hash of an artifact (code snapshot, evidence bundle,
    /// dataset, environment manifest, etc.).
    ArtifactHash
}

define_id! {
    /// Unique identifier for a validator.
    ValidatorId
}

define_id! {
    /// Unique identifier for a proposer.
    ProposerId
}

define_id! {
    /// Unique identifier for an escrow record.
    EscrowId
}

define_id! {
    /// Unique identifier for a domain specification.
    DomainSpecId
}

define_id! {
    /// General participant identifier.
    ///
    /// Used where the protocol role is not restricted to a specific type
    /// (e.g., challengers, escrow beneficiaries).
    ParticipantId
}

// ---------------------------------------------------------------------------
// Epoch identifier (sequential, not content-addressed)
// ---------------------------------------------------------------------------

/// Unique identifier for a protocol epoch.
///
/// Epochs are sequential intervals, so this wraps a `u64` rather than a
/// content-addressed hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EpochId(pub u64);

impl EpochId {
    /// The genesis epoch (epoch 0).
    pub const GENESIS: Self = Self(0);

    /// Return the next sequential epoch.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl std::fmt::Display for EpochId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "epoch:{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Cross-type conversions
// ---------------------------------------------------------------------------

impl GenesisBlockId {
    /// Interpret this genesis block ID as a regular [`BlockId`].
    ///
    /// Genesis blocks serve as root parents in the block chain. The first
    /// regular block in a track references the genesis block's ID as its
    /// parent via this conversion.
    pub fn as_block_id(&self) -> BlockId {
        BlockId(self.0)
    }

    /// Derive the [`TrackTreeId`] from this genesis block.
    ///
    /// A track tree is uniquely identified by its root genesis block.
    pub fn as_track_tree_id(&self) -> TrackTreeId {
        TrackTreeId(self.0)
    }
}

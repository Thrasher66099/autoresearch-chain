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

//! Content-addressed hashing for protocol artifacts.
//!
//! All artifact references in the protocol ([`ArtifactHash`]) are derived
//! from content bytes using BLAKE3. This module is the canonical source
//! of that computation.

use arc_protocol_types::ArtifactHash;

/// Compute the content-addressed hash of arbitrary bytes.
///
/// Uses BLAKE3, producing a 32-byte hash that maps directly to
/// [`ArtifactHash`]. The hash is computed over the raw content bytes
/// with no domain separation — two artifacts with identical content
/// bytes produce the same hash regardless of their [`ArtifactKind`].
///
/// This is the canonical hashing function for all content-addressed
/// references in the protocol.
///
/// [`ArtifactKind`]: crate::ArtifactKind
pub fn content_hash(data: &[u8]) -> ArtifactHash {
    let hash = blake3::hash(data);
    ArtifactHash::from_bytes(*hash.as_bytes())
}

/// Verify that content bytes match a claimed hash.
///
/// Returns `true` if `content_hash(content) == *claimed`. This is
/// the operation validators and challengers use to verify artifact
/// integrity: given content fetched from the artifact layer, does
/// it match the on-chain reference?
pub fn verify_content(content: &[u8], claimed: &ArtifactHash) -> bool {
    content_hash(content) == *claimed
}

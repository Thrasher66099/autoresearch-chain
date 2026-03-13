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

//! Artifact store trait and in-memory implementation.
//!
//! The artifact store is the protocol's interface for content-addressed
//! storage. All [`ArtifactHash`] references resolve against an artifact
//! store.

use std::collections::HashMap;

use arc_protocol_types::ArtifactHash;

use crate::artifact::{ArtifactKind, ArtifactMetadata};
use crate::error::StoreError;
use crate::hash::content_hash;

/// Content-addressed artifact storage.
///
/// This is the protocol's interface for storing and retrieving artifacts
/// by their content-addressed hash. All protocol references
/// ([`ArtifactHash`]) resolve against an artifact store.
///
/// Stores are content-addressed: the hash is computed from the content
/// bytes, not assigned externally. Storing the same content twice is
/// idempotent — the second store is a no-op.
///
/// # Invariants
///
/// Implementations must guarantee:
///
/// - After `store(data, ..)` returns `Ok(hash)`, `fetch(&hash)` returns
///   `Ok(Some(data))`.
/// - `store(data, ..)` is idempotent for identical content bytes.
/// - For any stored artifact, `content_hash(data) == hash`.
/// - `contains(&hash)` returns `true` if and only if `fetch(&hash)`
///   would return `Ok(Some(..))`.
pub trait ArtifactStore {
    /// Store content bytes and return the content-addressed hash.
    ///
    /// If the content already exists (same hash), this is a no-op and
    /// returns the existing hash. The metadata from the first store is
    /// preserved.
    fn store(
        &mut self,
        content: &[u8],
        kind: ArtifactKind,
        timestamp: u64,
    ) -> Result<ArtifactHash, StoreError>;

    /// Retrieve content bytes by hash.
    ///
    /// Returns `Ok(None)` if the artifact is not in the store.
    fn fetch(&self, hash: &ArtifactHash) -> Result<Option<Vec<u8>>, StoreError>;

    /// Check whether an artifact exists in the store.
    fn contains(&self, hash: &ArtifactHash) -> bool;

    /// Get metadata for a stored artifact.
    ///
    /// Returns `None` if the artifact is not in the store.
    fn metadata(&self, hash: &ArtifactHash) -> Option<&ArtifactMetadata>;
}

/// In-memory artifact store backed by `HashMap`.
///
/// Suitable for testing and for the local simulator. Not persistent —
/// contents are lost when the process exits. A future `FileArtifactStore`
/// or similar will back the node binary.
#[derive(Clone, Debug, Default)]
pub struct InMemoryArtifactStore {
    content: HashMap<ArtifactHash, Vec<u8>>,
    metadata: HashMap<ArtifactHash, ArtifactMetadata>,
}

impl InMemoryArtifactStore {
    /// Create an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of artifacts currently stored.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Whether the store contains no artifacts.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

impl ArtifactStore for InMemoryArtifactStore {
    fn store(
        &mut self,
        content: &[u8],
        kind: ArtifactKind,
        timestamp: u64,
    ) -> Result<ArtifactHash, StoreError> {
        let hash = content_hash(content);

        // Idempotent: if already stored, return existing hash.
        if self.content.contains_key(&hash) {
            return Ok(hash);
        }

        let meta = ArtifactMetadata {
            hash,
            kind,
            size_bytes: content.len() as u64,
            stored_at: timestamp,
        };

        self.content.insert(hash, content.to_vec());
        self.metadata.insert(hash, meta);

        Ok(hash)
    }

    fn fetch(&self, hash: &ArtifactHash) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self.content.get(hash).cloned())
    }

    fn contains(&self, hash: &ArtifactHash) -> bool {
        self.content.contains_key(hash)
    }

    fn metadata(&self, hash: &ArtifactHash) -> Option<&ArtifactMetadata> {
        self.metadata.get(hash)
    }
}

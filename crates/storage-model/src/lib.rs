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

//! Content-addressed artifact storage for AutoResearch Chain.
//!
//! The protocol references artifacts by their BLAKE3 content hash. This crate
//! provides the storage layer that maps between raw bytes and [`ArtifactHash`]
//! identifiers.
//!
//! # Architecture
//!
//! - [`ArtifactStore`] trait defines the store/fetch interface.
//! - [`InMemoryArtifactStore`] implements an in-memory store for testing and simulation.
//! - [`LocalContentStore`] implements file-backed storage in a local directory.
//! - [`EvidenceBundle`] groups the artifact hashes for a complete evidence submission.
//! - [`bundle_evidence`] is a convenience function to store multiple files at once.

pub mod artifact;
pub mod error;
pub mod hash;
pub mod store;

pub use artifact::{ArtifactKind, ArtifactMetadata};
pub use error::StoreError;
pub use hash::{content_hash, verify_content};
pub use store::{ArtifactStore, InMemoryArtifactStore};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use arc_protocol_types::ArtifactHash;

/// Convert an [`ArtifactHash`] to its hex-encoded string representation.
pub fn hash_to_hex(hash: &ArtifactHash) -> String {
    hash.as_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Parse a hex-encoded string into an [`ArtifactHash`].
pub fn hex_to_hash(hex: &str) -> Result<ArtifactHash, StoreError> {
    if hex.len() != 64 {
        return Err(StoreError::Internal {
            message: format!(
                "invalid hex hash length: expected 64, got {}",
                hex.len()
            ),
        });
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| StoreError::Internal {
                message: format!("invalid hex: {}", e),
            })?;
    }
    Ok(ArtifactHash::from_bytes(bytes))
}

/// File-backed content-addressed storage.
///
/// Stores artifacts as files named by their hex-encoded BLAKE3 hash
/// in a flat directory structure: `<store_dir>/<hex-hash>`.
pub struct LocalContentStore {
    /// Root directory for stored artifacts.
    store_dir: PathBuf,
    /// Metadata for stored artifacts.
    metadata: HashMap<ArtifactHash, ArtifactMetadata>,
}

impl LocalContentStore {
    /// Create a new store backed by the given directory.
    ///
    /// Creates the directory if it does not exist.
    pub fn new(store_dir: impl AsRef<Path>) -> Result<Self, StoreError> {
        let store_dir = store_dir.as_ref().to_path_buf();
        fs::create_dir_all(&store_dir)
            .map_err(|e| StoreError::Internal {
                message: format!("failed to create store dir: {}", e),
            })?;
        Ok(Self {
            store_dir,
            metadata: HashMap::new(),
        })
    }

    /// Return the filesystem path for a given artifact hash.
    pub fn artifact_path(&self, hash: &ArtifactHash) -> PathBuf {
        self.store_dir.join(hash_to_hex(hash))
    }

    /// Store a file from disk by reading and hashing its contents.
    pub fn store_file(&mut self, path: impl AsRef<Path>) -> Result<ArtifactHash, StoreError> {
        let data = fs::read(path.as_ref())
            .map_err(|e| StoreError::Internal {
                message: format!("failed to read {}: {}", path.as_ref().display(), e),
            })?;
        self.store(&data, ArtifactKind::SourceTree, 0)
    }
}

impl ArtifactStore for LocalContentStore {
    fn store(
        &mut self,
        data: &[u8],
        kind: ArtifactKind,
        timestamp: u64,
    ) -> Result<ArtifactHash, StoreError> {
        let hash = content_hash(data);
        let path = self.artifact_path(&hash);
        // Content-addressed: if it already exists, contents are identical.
        if !path.exists() {
            fs::write(&path, data)
                .map_err(|e| StoreError::Internal {
                    message: format!("failed to write artifact: {}", e),
                })?;
            self.metadata.insert(hash, ArtifactMetadata {
                hash,
                kind,
                size_bytes: data.len() as u64,
                stored_at: timestamp,
            });
        }
        Ok(hash)
    }

    fn fetch(&self, hash: &ArtifactHash) -> Result<Option<Vec<u8>>, StoreError> {
        let path = self.artifact_path(hash);
        if path.exists() {
            fs::read(&path)
                .map(Some)
                .map_err(|e| StoreError::Internal {
                    message: format!("failed to read artifact: {}", e),
                })
        } else {
            Ok(None)
        }
    }

    fn contains(&self, hash: &ArtifactHash) -> bool {
        self.artifact_path(hash).exists()
    }

    fn metadata(&self, hash: &ArtifactHash) -> Option<&ArtifactMetadata> {
        self.metadata.get(hash)
    }
}

/// A collection of content-addressed artifact hashes for a protocol block's
/// evidence submission.
///
/// Mirrors the evidence components required by the protocol: code diff,
/// resolved configuration, environment manifest, training logs, and metric
/// outputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBundle {
    /// Hash of the code diff (parent → child).
    pub diff_hash: ArtifactHash,
    /// Hash of the resolved training configuration.
    pub config_hash: ArtifactHash,
    /// Hash of the environment manifest (Python version, CUDA, GPU, deps).
    pub env_manifest_hash: ArtifactHash,
    /// Hash of the training logs.
    pub training_log_hash: ArtifactHash,
    /// Hash of the metric/eval outputs.
    pub metric_output_hash: ArtifactHash,
}

/// Store evidence files and return a bundle of their content-addressed hashes.
///
/// Reads each file, stores it in the content store, and collects the resulting
/// hashes into an [`EvidenceBundle`].
pub fn bundle_evidence(
    store: &mut impl ArtifactStore,
    diff_path: impl AsRef<Path>,
    config_path: impl AsRef<Path>,
    env_manifest_path: impl AsRef<Path>,
    training_log_path: impl AsRef<Path>,
    metric_output_path: impl AsRef<Path>,
    timestamp: u64,
) -> Result<EvidenceBundle, StoreError> {
    let read_and_store = |store: &mut dyn ArtifactStore, path: &Path, kind: ArtifactKind| -> Result<ArtifactHash, StoreError> {
        let data = fs::read(path)
            .map_err(|e| StoreError::Internal {
                message: format!("failed to read {}: {}", path.display(), e),
            })?;
        store.store(&data, kind, timestamp)
    };

    Ok(EvidenceBundle {
        diff_hash: read_and_store(store, diff_path.as_ref(), ArtifactKind::Diff)?,
        config_hash: read_and_store(store, config_path.as_ref(), ArtifactKind::Config)?,
        env_manifest_hash: read_and_store(store, env_manifest_path.as_ref(), ArtifactKind::EnvironmentManifest)?,
        training_log_hash: read_and_store(store, training_log_path.as_ref(), ArtifactKind::TrainingLog)?,
        metric_output_hash: read_and_store(store, metric_output_path.as_ref(), ArtifactKind::MetricOutput)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_store() -> (TempDir, LocalContentStore) {
        let tmp = TempDir::new().unwrap();
        let store = LocalContentStore::new(tmp.path().join("artifacts")).unwrap();
        (tmp, store)
    }

    #[test]
    fn store_and_fetch_roundtrip() {
        let (_tmp, mut store) = setup_store();
        let data = b"hello, world!";
        let hash = store.store(data, ArtifactKind::Diff, 0).unwrap();

        // Hash should be non-zero.
        assert_ne!(hash, ArtifactHash::ZERO);

        // Fetch should return original data.
        let fetched = store.fetch(&hash).unwrap();
        assert_eq!(fetched, Some(data.to_vec()));
    }

    #[test]
    fn hash_is_deterministic() {
        let data = b"deterministic content";
        let hash1 = content_hash(data);
        let hash2 = content_hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn different_content_different_hash() {
        let hash1 = content_hash(b"content A");
        let hash2 = content_hash(b"content B");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn store_is_idempotent() {
        let (_tmp, mut store) = setup_store();
        let data = b"same data twice";
        let hash1 = store.store(data, ArtifactKind::Diff, 0).unwrap();
        let hash2 = store.store(data, ArtifactKind::Diff, 0).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn fetch_missing_artifact_returns_none() {
        let (_tmp, store) = setup_store();
        let fake_hash = ArtifactHash::from_bytes([42u8; 32]);
        let result = store.fetch(&fake_hash).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn contains_check() {
        let (_tmp, mut store) = setup_store();
        let hash = store.store(b"exists test", ArtifactKind::Diff, 0).unwrap();
        assert!(store.contains(&hash));

        let missing = ArtifactHash::from_bytes([99u8; 32]);
        assert!(!store.contains(&missing));
    }

    #[test]
    fn hex_roundtrip() {
        let hash = content_hash(b"hex test");
        let hex = hash_to_hex(&hash);
        assert_eq!(hex.len(), 64);
        let parsed = hex_to_hash(&hex).unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn store_file_from_disk() {
        let tmp = TempDir::new().unwrap();
        let mut store = LocalContentStore::new(tmp.path().join("artifacts")).unwrap();

        // Write a test file.
        let file_path = tmp.path().join("test.txt");
        fs::write(&file_path, b"file content for hashing").unwrap();

        let hash = store.store_file(&file_path).unwrap();
        assert_ne!(hash, ArtifactHash::ZERO);

        // Verify contents match.
        let fetched = store.fetch(&hash).unwrap();
        assert_eq!(fetched, Some(b"file content for hashing".to_vec()));

        // Hash should match direct computation.
        let expected_hash = content_hash(b"file content for hashing");
        assert_eq!(hash, expected_hash);
    }

    #[test]
    fn bundle_evidence_stores_all_files() {
        let tmp = TempDir::new().unwrap();
        let mut store = LocalContentStore::new(tmp.path().join("artifacts")).unwrap();

        // Create test evidence files.
        let diff = tmp.path().join("diff.patch");
        let config = tmp.path().join("config.yaml");
        let env = tmp.path().join("env.json");
        let log = tmp.path().join("train.log");
        let metrics = tmp.path().join("metrics.json");

        fs::write(&diff, b"--- a/train.py\n+++ b/train.py\n@@ -1 +1 @@\n-old\n+new").unwrap();
        fs::write(&config, b"lr: 0.001\nepochs: 5").unwrap();
        fs::write(&env, b"{\"python\": \"3.10\", \"cuda\": \"12.1\"}").unwrap();
        fs::write(&log, b"epoch 1/5: loss=0.5\nepoch 2/5: loss=0.3").unwrap();
        fs::write(&metrics, b"{\"test_accuracy\": 0.945}").unwrap();

        let bundle = bundle_evidence(&mut store, &diff, &config, &env, &log, &metrics, 0).unwrap();

        // All hashes should be non-zero and distinct.
        assert_ne!(bundle.diff_hash, ArtifactHash::ZERO);
        assert_ne!(bundle.config_hash, ArtifactHash::ZERO);
        assert_ne!(bundle.env_manifest_hash, ArtifactHash::ZERO);
        assert_ne!(bundle.training_log_hash, ArtifactHash::ZERO);
        assert_ne!(bundle.metric_output_hash, ArtifactHash::ZERO);

        // All hashes should be retrievable.
        assert!(store.contains(&bundle.diff_hash));
        assert!(store.contains(&bundle.config_hash));
        assert!(store.contains(&bundle.env_manifest_hash));
        assert!(store.contains(&bundle.training_log_hash));
        assert!(store.contains(&bundle.metric_output_hash));
    }

    #[test]
    fn bundle_evidence_fails_on_missing_file() {
        let tmp = TempDir::new().unwrap();
        let mut store = LocalContentStore::new(tmp.path().join("artifacts")).unwrap();

        let existing = tmp.path().join("exists.txt");
        fs::write(&existing, b"content").unwrap();

        let missing = tmp.path().join("does_not_exist.txt");

        let result = bundle_evidence(&mut store, &missing, &existing, &existing, &existing, &existing, 0);
        assert!(result.is_err());
    }
}

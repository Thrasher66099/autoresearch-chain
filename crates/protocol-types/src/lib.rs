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

//! Core protocol types for AutoResearch Chain.
//!
//! This crate defines the foundational data structures used across the protocol.
//! It contains identifiers, blocks, domains, tracks, forks, challenges, rewards,
//! and canonical state references.
//!
//! These are structural definitions only. State transition logic lives in
//! `arc-protocol-rules`. Domain lifecycle lives in `arc-domain-engine`.
//! Fork logic lives in `arc-fork-engine`. And so on.
//!
//! # Module organization
//!
//! - [`ids`] --- Strongly typed identifier and reference wrappers
//! - [`enums`] --- Canonical protocol enums (statuses, classifications, votes)
//! - [`metric`] --- Protocol metric value abstraction ([`MetricValue`])
//! - [`token`] --- Protocol token amount abstraction ([`TokenAmount`])
//! - [`domain`] --- Problem domain and domain specification
//! - [`genesis`] --- Genesis block, research track standards, track initialization
//! - [`block`] --- Block and epoch types
//! - [`validation`] --- Validation attestations and evidence bundles
//! - [`challenge`] --- Challenge records and challenge target modeling
//! - [`frontier`] --- Canonical frontier state, materialized state, codebase references
//! - [`fork`] --- Fork families
//! - [`escrow`] --- Escrow records and attribution claims
//! - [`policy`] --- Metric and dataset integrity policies
//!
//! All public types are re-exported at the crate root for convenience.

pub mod ids;
pub mod enums;
pub mod metric;
pub mod token;
pub mod domain;
pub mod genesis;
pub mod block;
pub mod validation;
pub mod challenge;
pub mod frontier;
pub mod fork;
pub mod escrow;
pub mod policy;
pub mod validate;
pub mod fixtures;

// Re-export all public types at crate root for convenience.
// Users can write `use arc_protocol_types::BlockId;` or
// `use arc_protocol_types::ids::BlockId;` --- both work.
pub use ids::*;
pub use enums::*;
pub use metric::*;
pub use token::*;
pub use domain::*;
pub use genesis::*;
pub use block::*;
pub use validation::*;
pub use challenge::*;
pub use frontier::*;
pub use fork::*;
pub use escrow::*;
pub use policy::*;
pub use validate::*;
pub use fixtures::*;

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // ID wrapper tests
    // -----------------------------------------------------------------------

    #[test]
    fn id_zero_is_all_zeros() {
        assert_eq!(DomainId::ZERO.as_bytes(), &[0u8; 32]);
        assert_eq!(BlockId::ZERO.as_bytes(), &[0u8; 32]);
        assert_eq!(GenesisBlockId::ZERO.as_bytes(), &[0u8; 32]);
        assert_eq!(ArtifactHash::ZERO.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn id_from_bytes_roundtrip() {
        let bytes = [42u8; 32];
        let id = DomainId::from_bytes(bytes);
        assert_eq!(*id.as_bytes(), bytes);
    }

    #[test]
    fn different_id_types_do_not_conflate() {
        // Type safety: DomainId and BlockId have the same bytes but are
        // distinct types. This test verifies they exist independently.
        let d = DomainId::ZERO;
        let b = BlockId::ZERO;
        // Same underlying bytes, but `d == b` would not compile.
        assert_eq!(d.as_bytes(), b.as_bytes());
    }

    #[test]
    fn id_display_shows_hex_prefix() {
        let id = DomainId::from_bytes({
            let mut b = [0u8; 32];
            b[0] = 0xab;
            b[1] = 0xcd;
            b[2] = 0xef;
            b[3] = 0x01;
            b
        });
        let displayed = format!("{}", id);
        assert_eq!(displayed, "abcdef01\u{2026}");
    }

    #[test]
    fn id_debug_includes_type_name() {
        let id = BlockId::from_bytes([0xff; 32]);
        let debug = format!("{:?}", id);
        assert!(debug.starts_with("BlockId("));
    }

    #[test]
    fn epoch_id_ordering() {
        let e0 = EpochId::GENESIS;
        let e1 = e0.next();
        let e2 = e1.next();
        assert!(e0 < e1);
        assert!(e1 < e2);
        assert_eq!(e2.0, 2);
    }

    #[test]
    fn epoch_id_display() {
        assert_eq!(format!("{}", EpochId::GENESIS), "epoch:0");
        assert_eq!(format!("{}", EpochId(42)), "epoch:42");
    }

    #[test]
    fn genesis_block_id_conversions() {
        let gid = GenesisBlockId::from_bytes([1u8; 32]);
        let bid = gid.as_block_id();
        let tid = gid.as_track_tree_id();
        assert_eq!(gid.as_bytes(), bid.as_bytes());
        assert_eq!(gid.as_bytes(), tid.as_bytes());
    }

    // -----------------------------------------------------------------------
    // Enum variant distinctness
    // -----------------------------------------------------------------------

    #[test]
    fn block_status_variants_are_distinct() {
        let statuses = [
            BlockStatus::Submitted,
            BlockStatus::UnderValidation,
            BlockStatus::ValidationComplete,
            BlockStatus::UnderChallenge,
            BlockStatus::ChallengeWindowClosed,
            BlockStatus::Settled,
            BlockStatus::Final,
            BlockStatus::Rejected,
            BlockStatus::Invalidated,
        ];
        for (i, a) in statuses.iter().enumerate() {
            for (j, b) in statuses.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn challenge_type_variants_are_distinct() {
        assert_ne!(ChallengeType::BlockReplay, ChallengeType::AttestationFraud);
        assert_ne!(ChallengeType::Attribution, ChallengeType::Dominance);
        assert_ne!(ChallengeType::Dominance, ChallengeType::MetricAdequacy);
    }

    #[test]
    fn validator_vote_variants_are_distinct() {
        assert_ne!(ValidatorVote::Pass, ValidatorVote::Fail);
        assert_ne!(ValidatorVote::Inconclusive, ValidatorVote::FraudSuspected);
    }

    #[test]
    fn metric_direction_variants_are_distinct() {
        assert_ne!(MetricDirection::HigherBetter, MetricDirection::LowerBetter);
    }

    // -----------------------------------------------------------------------
    // Serialization round-trips
    // -----------------------------------------------------------------------

    #[test]
    fn serde_roundtrip_domain_id() {
        let id = DomainId::from_bytes([7u8; 32]);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: DomainId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, recovered);
    }

    #[test]
    fn serde_roundtrip_epoch_id() {
        let eid = EpochId(42);
        let json = serde_json::to_string(&eid).unwrap();
        let recovered: EpochId = serde_json::from_str(&json).unwrap();
        assert_eq!(eid, recovered);
    }

    #[test]
    fn serde_roundtrip_block_status() {
        let status = BlockStatus::UnderValidation;
        let json = serde_json::to_string(&status).unwrap();
        let recovered: BlockStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, recovered);
    }

    #[test]
    fn serde_roundtrip_problem_domain() {
        let domain = ProblemDomain {
            id: DomainId::ZERO,
            name: "test-domain".to_string(),
            domain_type: DomainType::Experimental,
            parent_domain_id: None,
            spec_id: DomainSpecId::ZERO,
        };
        let json = serde_json::to_string(&domain).unwrap();
        let recovered: ProblemDomain = serde_json::from_str(&json).unwrap();
        assert_eq!(domain, recovered);
    }

    #[test]
    fn serde_roundtrip_validation_attestation() {
        let att = ValidationAttestation {
            block_id: BlockId::ZERO,
            validator: ValidatorId::ZERO,
            vote: ValidatorVote::Pass,
            observed_delta: Some(MetricValue::new(0.01)),
            replay_evidence_ref: ArtifactHash::ZERO,
            timestamp: 1000,
        };
        let json = serde_json::to_string(&att).unwrap();
        let recovered: ValidationAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(att, recovered);
    }

    #[test]
    fn serde_roundtrip_codebase_state_ref_variants() {
        let refs = vec![
            CodebaseStateRef::LatestFrontier {
                domain_id: DomainId::ZERO,
            },
            CodebaseStateRef::Historical {
                materialized_state_id: MaterializedStateId::ZERO,
            },
            CodebaseStateRef::AtBlock {
                block_id: BlockId::ZERO,
            },
        ];
        for r in &refs {
            let json = serde_json::to_string(r).unwrap();
            let recovered: CodebaseStateRef = serde_json::from_str(&json).unwrap();
            assert_eq!(r, &recovered);
        }
    }

    #[test]
    fn serde_roundtrip_escrow_record() {
        let escrow = EscrowRecord {
            id: EscrowId::ZERO,
            block_id: BlockId::ZERO,
            beneficiary: ParticipantId::ZERO,
            amount: TokenAmount::new(100),
            status: EscrowStatus::Held,
            created_epoch: EpochId(1),
            release_epoch: EpochId(5),
        };
        let json = serde_json::to_string(&escrow).unwrap();
        let recovered: EscrowRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(escrow, recovered);
    }

    // -----------------------------------------------------------------------
    // Struct construction tests
    // -----------------------------------------------------------------------

    #[test]
    fn construct_genesis_block() {
        let genesis = GenesisBlock {
            id: GenesisBlockId::ZERO,
            rts_version: ResearchTrackStandardVersion::Rts1,
            domain_id: DomainId::ZERO,
            proposer: ProposerId::ZERO,
            research_target_declaration: "Improve CIFAR-10 training recipe".to_string(),
            domain_intent: DomainIntent::EndToEndRecipeImprovement,
            seed_recipe_ref: ArtifactHash::ZERO,
            seed_codebase_state_ref: ArtifactHash::ZERO,
            frozen_surface: vec!["eval/".to_string()],
            search_surface: vec!["train.py".to_string(), "config/".to_string()],
            canonical_dataset_ref: ArtifactHash::ZERO,
            dataset_hash: ArtifactHash::ZERO,
            dataset_splits: DatasetSplits {
                training: ArtifactHash::ZERO,
                validation: ArtifactHash::ZERO,
                test: Some(ArtifactHash::ZERO),
            },
            evaluation_harness_ref: ArtifactHash::ZERO,
            metric_id: "test_accuracy".to_string(),
            metric_direction: MetricDirection::HigherBetter,
            hardware_class: "RTX 4090".to_string(),
            time_budget_secs: 3600,
            seed_environment_manifest_ref: ArtifactHash::ZERO,
            seed_score: MetricValue::new(0.93),
            artifact_schema_ref: ArtifactHash::ZERO,
            seed_bond: TokenAmount::new(1000),
            license_declaration: "MIT".to_string(),
            timestamp: 1700000000,
        };
        assert_eq!(genesis.rts_version, ResearchTrackStandardVersion::Rts1);
        assert_eq!(genesis.metric_direction, MetricDirection::HigherBetter);
    }

    #[test]
    fn construct_block() {
        let block = Block {
            id: BlockId::from_bytes([1u8; 32]),
            domain_id: DomainId::ZERO,
            parent_id: BlockId::ZERO,
            proposer: ProposerId::ZERO,
            child_state_ref: ArtifactHash::from_bytes([2u8; 32]),
            diff_ref: ArtifactHash::from_bytes([3u8; 32]),
            claimed_metric_delta: MetricValue::new(0.02),
            evidence_bundle_hash: ArtifactHash::from_bytes([4u8; 32]),
            fee: TokenAmount::new(10),
            bond: TokenAmount::new(100),
            epoch_id: EpochId(1),
            status: BlockStatus::Submitted,
            timestamp: 1700000100,
        };
        assert_eq!(block.status, BlockStatus::Submitted);
        assert_eq!(block.epoch_id, EpochId(1));
    }

    #[test]
    fn construct_challenge_record() {
        let challenge = ChallengeRecord {
            id: ChallengeId::ZERO,
            challenge_type: ChallengeType::BlockReplay,
            target: ChallengeTarget::Block {
                block_id: BlockId::ZERO,
            },
            challenger: ParticipantId::ZERO,
            bond: TokenAmount::new(500),
            evidence_ref: ArtifactHash::ZERO,
            status: ChallengeStatus::Open,
            epoch_id: EpochId(2),
            timestamp: 1700000200,
        };
        assert_eq!(challenge.status, ChallengeStatus::Open);
    }

    #[test]
    fn construct_fork_family() {
        let fork = ForkFamily {
            id: ForkFamilyId::ZERO,
            domain_id: DomainId::ZERO,
            track_tree_id: TrackTreeId::ZERO,
            common_ancestor_id: BlockId::ZERO,
            branch_tips: vec![
                BlockId::from_bytes([1u8; 32]),
                BlockId::from_bytes([2u8; 32]),
            ],
            dominant_branch_tip: None,
        };
        assert_eq!(fork.branch_tips.len(), 2);
        assert!(fork.dominant_branch_tip.is_none());
    }

    #[test]
    fn construct_track_tree() {
        let gid = GenesisBlockId::from_bytes([10u8; 32]);
        let tree = TrackTree {
            id: gid.as_track_tree_id(),
            domain_id: DomainId::ZERO,
            genesis_block_id: gid,
            fork_families: vec![],
            canonical_frontier_block_id: None,
        };
        assert_eq!(tree.genesis_block_id.as_bytes(), tree.id.as_bytes());
        assert!(tree.fork_families.is_empty());
    }

    #[test]
    fn construct_materialized_state() {
        let ms = MaterializedState {
            id: MaterializedStateId::ZERO,
            domain_id: DomainId::ZERO,
            root_tree_hash: ArtifactHash::from_bytes([1u8; 32]),
            resolved_dependency_manifest_hash: ArtifactHash::from_bytes([2u8; 32]),
            resolved_config_hash: ArtifactHash::from_bytes([3u8; 32]),
            environment_manifest_hash: ArtifactHash::from_bytes([4u8; 32]),
            evaluation_manifest_hash: ArtifactHash::from_bytes([5u8; 32]),
            materialized_from_block_id: BlockId::ZERO,
            timestamp: 1700000300,
        };
        assert_ne!(ms.root_tree_hash, ms.resolved_config_hash);
    }

    #[test]
    fn construct_escrow_record() {
        let escrow = EscrowRecord {
            id: EscrowId::ZERO,
            block_id: BlockId::ZERO,
            beneficiary: ParticipantId::ZERO,
            amount: TokenAmount::new(100),
            status: EscrowStatus::Held,
            created_epoch: EpochId(1),
            release_epoch: EpochId(5),
        };
        assert_eq!(escrow.status, EscrowStatus::Held);
        assert!(escrow.release_epoch > escrow.created_epoch);
    }
}

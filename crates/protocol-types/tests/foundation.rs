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

//! Foundation test checklist for Phase 0.1 protocol types.
//!
//! These tests verify structural correctness of the protocol type model
//! before any state-machine logic is built on top of it. They cover:
//!
//! - **Group A**: ID type separation and constructor behavior
//! - **Group B**: Enum round-trip serialization and naming stability
//! - **Group C**: GenesisBlock structural validation
//! - **Group D**: Block structural validation
//! - **Group E**: ValidationAttestation structural validation
//! - **Group F**: ChallengeRecord structural validation
//! - **Group G**: Canonical fixture integrity (covered by C-F + H)
//! - **Group H**: Serialization determinism for all major structs
//! - **Group I**: Invariant helper coverage (covered by C-F)

use std::collections::HashSet;

use arc_protocol_types::*;

// =======================================================================
// Group A — ID and type separation
// =======================================================================

/// All 13 newtype ID types are distinct at the type level.
///
/// This test exists to confirm that the macro-generated types compile
/// independently and that their ZERO constants are available. If two
/// ID types were accidentally aliased, this would fail to compile or
/// produce collisions.
#[test]
fn a_all_id_types_have_independent_zero() {
    // Each type's ZERO is independently addressable. If the macro
    // produced duplicate type names, this would not compile.
    let _ = DomainId::ZERO;
    let _ = BlockId::ZERO;
    let _ = GenesisBlockId::ZERO;
    let _ = ForkFamilyId::ZERO;
    let _ = TrackTreeId::ZERO;
    let _ = ChallengeId::ZERO;
    let _ = MaterializedStateId::ZERO;
    let _ = ArtifactHash::ZERO;
    let _ = ValidatorId::ZERO;
    let _ = ProposerId::ZERO;
    let _ = EscrowId::ZERO;
    let _ = DomainSpecId::ZERO;
    let _ = ParticipantId::ZERO;
}

/// from_bytes/as_bytes round-trips correctly for all ID types.
#[test]
fn a_id_from_bytes_roundtrip_all_types() {
    let bytes = [0xABu8; 32];

    assert_eq!(*DomainId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*BlockId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*GenesisBlockId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*ForkFamilyId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*TrackTreeId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*ChallengeId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*MaterializedStateId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*ArtifactHash::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*ValidatorId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*ProposerId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*EscrowId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*DomainSpecId::from_bytes(bytes).as_bytes(), bytes);
    assert_eq!(*ParticipantId::from_bytes(bytes).as_bytes(), bytes);
}

/// Hash trait is consistent with Eq for ID types.
///
/// Two IDs with the same bytes must have the same hash; two with
/// different bytes must not collide (with overwhelmingly high
/// probability for 32-byte values).
#[test]
fn a_id_hash_consistent_with_eq() {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    fn compute_hash<T: Hash>(val: &T) -> u64 {
        let mut h = DefaultHasher::new();
        val.hash(&mut h);
        h.finish()
    }

    let a = DomainId::from_bytes([1u8; 32]);
    let b = DomainId::from_bytes([1u8; 32]);
    let c = DomainId::from_bytes([2u8; 32]);

    assert_eq!(a, b);
    assert_eq!(compute_hash(&a), compute_hash(&b));
    assert_ne!(a, c);
    // Hash collision is theoretically possible but essentially impossible
    // for 32-byte differing inputs through DefaultHasher.
    assert_ne!(compute_hash(&a), compute_hash(&c));
}

/// GenesisBlockId can be used as both a BlockId and a TrackTreeId.
/// The byte values are preserved through conversion.
#[test]
fn a_genesis_block_id_cross_type_conversions() {
    let gid = GenesisBlockId::from_bytes([42u8; 32]);
    let bid = gid.as_block_id();
    let tid = gid.as_track_tree_id();

    assert_eq!(gid.as_bytes(), bid.as_bytes());
    assert_eq!(gid.as_bytes(), tid.as_bytes());
}

/// GenesisBlockId and BlockId are not interchangeable at the type level.
///
/// This is a documentation-level test: you cannot pass a GenesisBlockId
/// where a BlockId is expected without explicit conversion. The explicit
/// `.as_block_id()` conversion exists for that purpose.
#[test]
fn a_genesis_and_block_id_require_explicit_conversion() {
    let gid = GenesisBlockId::from_bytes([1u8; 32]);
    let bid = BlockId::from_bytes([1u8; 32]);

    // Same bytes, but different types. Direct equality does not compile:
    //   assert_eq!(gid, bid);  // Would not compile.
    // The intended path is explicit conversion:
    assert_eq!(gid.as_block_id(), bid);
}

/// EpochId supports sequential ordering and the next() method.
#[test]
fn a_epoch_id_sequential_behavior() {
    let e0 = EpochId::GENESIS;
    assert_eq!(e0.0, 0);

    let e1 = e0.next();
    let e2 = e1.next();
    let e3 = e2.next();

    assert!(e0 < e1);
    assert!(e1 < e2);
    assert!(e2 < e3);
    assert_eq!(e3.0, 3);

    // EpochId is Ord, so it works in sorted collections.
    let mut epochs = vec![e3, e0, e2, e1];
    epochs.sort();
    assert_eq!(epochs, vec![e0, e1, e2, e3]);
}

/// DomainId cannot be stored in a HashSet<BlockId> (type safety).
///
/// This test verifies that the newtype pattern works for collection
/// type safety. It's a documentation test — the commented-out code
/// would not compile.
#[test]
fn a_id_type_safety_in_collections() {
    let mut block_ids: HashSet<BlockId> = HashSet::new();
    block_ids.insert(BlockId::from_bytes([1u8; 32]));

    // This would not compile:
    //   block_ids.insert(DomainId::from_bytes([1u8; 32]));

    // Only BlockId values can be looked up:
    assert!(block_ids.contains(&BlockId::from_bytes([1u8; 32])));
}

// =======================================================================
// Group B — Enum correctness
// =======================================================================

/// Helper: serialize to JSON, deserialize back, and verify equality.
fn enum_roundtrip<T>(val: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(val).expect("serialize");
    let recovered: T = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(val, &recovered, "round-trip failed for JSON: {}", json);
}

#[test]
fn b_block_status_all_variants_roundtrip() {
    let variants = [
        BlockStatus::Submitted,
        BlockStatus::UnderValidation,
        BlockStatus::ValidationComplete,
        BlockStatus::UnderChallenge,
        BlockStatus::ChallengeWindowClosed,
        BlockStatus::Settled,
        BlockStatus::Final,
        BlockStatus::Rejected,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_challenge_status_all_variants_roundtrip() {
    let variants = [
        ChallengeStatus::Open,
        ChallengeStatus::UnderReview,
        ChallengeStatus::Upheld,
        ChallengeStatus::Rejected,
        ChallengeStatus::Expired,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_challenge_type_all_variants_roundtrip() {
    let variants = [
        ChallengeType::BlockReplay,
        ChallengeType::AttestationFraud,
        ChallengeType::Attribution,
        ChallengeType::Dominance,
        ChallengeType::MetricAdequacy,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_validator_vote_all_variants_roundtrip() {
    let variants = [
        ValidatorVote::Pass,
        ValidatorVote::Fail,
        ValidatorVote::Inconclusive,
        ValidatorVote::FraudSuspected,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_metric_direction_all_variants_roundtrip() {
    let variants = [MetricDirection::HigherBetter, MetricDirection::LowerBetter];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_domain_type_all_variants_roundtrip() {
    let variants = [
        DomainType::Root,
        DomainType::Model,
        DomainType::Subsystem,
        DomainType::Technique,
        DomainType::Infrastructure,
        DomainType::Integration,
        DomainType::Experimental,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_domain_intent_all_variants_roundtrip() {
    let variants = [
        DomainIntent::EndToEndRecipeImprovement,
        DomainIntent::SubsystemOptimization,
        DomainIntent::TransferableOptimizerResearch,
        DomainIntent::InfrastructureEfficiency,
        DomainIntent::ConsumerGpuTrainingEfficiency,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_rts_version_all_variants_roundtrip() {
    let variants = [ResearchTrackStandardVersion::Rts1];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_track_activation_state_all_variants_roundtrip() {
    let variants = [
        TrackActivationState::Proposed,
        TrackActivationState::ConformanceChecking,
        TrackActivationState::ValidationInProgress,
        TrackActivationState::ActivationPending,
        TrackActivationState::Active,
        TrackActivationState::Failed,
        TrackActivationState::Expired,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_frontier_status_all_variants_roundtrip() {
    let variants = [
        FrontierStatus::Active,
        FrontierStatus::Contested,
        FrontierStatus::Settled,
        FrontierStatus::Superseded,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_materialization_policy_all_variants_roundtrip() {
    let variants = [
        MaterializationPolicyKind::OnDominance,
        MaterializationPolicyKind::Scheduled,
        MaterializationPolicyKind::DiffChainThreshold,
        MaterializationPolicyKind::Manual,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_attribution_type_all_variants_roundtrip() {
    let variants = [
        AttributionType::Origin,
        AttributionType::Integration,
        AttributionType::Frontier,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

#[test]
fn b_escrow_status_all_variants_roundtrip() {
    let variants = [
        EscrowStatus::Held,
        EscrowStatus::Released,
        EscrowStatus::Slashed,
    ];
    for v in &variants {
        enum_roundtrip(v);
    }
}

/// Verify that serde JSON representations of enums use stable, readable
/// names. This catches accidental renames or serialization format changes.
#[test]
fn b_enum_json_names_are_stable() {
    // Spot-check key enums for their expected JSON string forms.
    assert_eq!(
        serde_json::to_string(&BlockStatus::Submitted).unwrap(),
        "\"Submitted\""
    );
    assert_eq!(
        serde_json::to_string(&BlockStatus::ChallengeWindowClosed).unwrap(),
        "\"ChallengeWindowClosed\""
    );
    assert_eq!(
        serde_json::to_string(&ValidatorVote::FraudSuspected).unwrap(),
        "\"FraudSuspected\""
    );
    assert_eq!(
        serde_json::to_string(&ChallengeType::AttestationFraud).unwrap(),
        "\"AttestationFraud\""
    );
    assert_eq!(
        serde_json::to_string(&MetricDirection::HigherBetter).unwrap(),
        "\"HigherBetter\""
    );
    assert_eq!(
        serde_json::to_string(&ResearchTrackStandardVersion::Rts1).unwrap(),
        "\"Rts1\""
    );
    assert_eq!(
        serde_json::to_string(&DomainIntent::EndToEndRecipeImprovement).unwrap(),
        "\"EndToEndRecipeImprovement\""
    );
    assert_eq!(
        serde_json::to_string(&FrontierStatus::Superseded).unwrap(),
        "\"Superseded\""
    );
}

/// All enum variants within a type are distinct (no duplicate discriminants).
#[test]
fn b_enum_variants_all_distinct() {
    // BlockStatus: all 8 variants pairwise distinct
    let statuses = [
        BlockStatus::Submitted,
        BlockStatus::UnderValidation,
        BlockStatus::ValidationComplete,
        BlockStatus::UnderChallenge,
        BlockStatus::ChallengeWindowClosed,
        BlockStatus::Settled,
        BlockStatus::Final,
        BlockStatus::Rejected,
    ];
    for (i, a) in statuses.iter().enumerate() {
        for (j, b) in statuses.iter().enumerate() {
            assert_eq!(i == j, a == b, "BlockStatus mismatch at ({}, {})", i, j);
        }
    }

    // ValidatorVote: all 4 variants pairwise distinct
    let votes = [
        ValidatorVote::Pass,
        ValidatorVote::Fail,
        ValidatorVote::Inconclusive,
        ValidatorVote::FraudSuspected,
    ];
    for (i, a) in votes.iter().enumerate() {
        for (j, b) in votes.iter().enumerate() {
            assert_eq!(i == j, a == b, "ValidatorVote mismatch at ({}, {})", i, j);
        }
    }
}

// =======================================================================
// Group C — GenesisBlock structural validity
// =======================================================================

#[test]
fn c_valid_genesis_passes_validation() {
    let g = valid_genesis_block();
    assert!(
        validate_genesis_block_structure(&g).is_ok(),
        "valid genesis fixture should pass structural validation"
    );
}

#[test]
fn c_genesis_missing_metric_id_fails() {
    let g = invalid_genesis_missing_metric_id();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(
        errs.iter().any(|e| e.field == "metric_id"),
        "expected metric_id error, got: {:?}",
        errs
    );
}

#[test]
fn c_genesis_missing_dataset_hash_fails() {
    let g = invalid_genesis_missing_dataset_hash();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "dataset_hash"));
}

#[test]
fn c_genesis_empty_search_surface_fails() {
    let g = invalid_genesis_empty_search_surface();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "search_surface"));
}

#[test]
fn c_genesis_empty_frozen_surface_fails() {
    let g = invalid_genesis_empty_frozen_surface();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "frozen_surface"));
}

#[test]
fn c_genesis_missing_research_target_fails() {
    let g = invalid_genesis_missing_research_target();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "research_target_declaration"));
}

#[test]
fn c_genesis_nan_seed_score_fails() {
    let g = invalid_genesis_nan_seed_score();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "seed_score"));
}

#[test]
fn c_genesis_inf_seed_score_fails() {
    let g = invalid_genesis_inf_seed_score();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "seed_score"));
}

#[test]
fn c_genesis_zero_time_budget_fails() {
    let g = invalid_genesis_zero_time_budget();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "time_budget_secs"));
}

#[test]
fn c_genesis_missing_hardware_class_fails() {
    let g = invalid_genesis_missing_hardware_class();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "hardware_class"));
}

#[test]
fn c_genesis_missing_eval_harness_fails() {
    let g = invalid_genesis_missing_eval_harness();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "evaluation_harness_ref"));
}

#[test]
fn c_genesis_missing_seed_recipe_fails() {
    let g = invalid_genesis_missing_seed_recipe();
    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "seed_recipe_ref"));
}

/// Multiple structural failures are reported together, not just the first.
#[test]
fn c_genesis_multiple_failures_reported() {
    let mut g = valid_genesis_block();
    g.metric_id = String::new();
    g.search_surface = vec![];
    g.time_budget_secs = 0;

    let errs = validate_genesis_block_structure(&g).unwrap_err();
    assert!(
        errs.len() >= 3,
        "expected at least 3 errors, got {}: {:?}",
        errs.len(),
        errs
    );
    assert!(errs.iter().any(|e| e.field == "metric_id"));
    assert!(errs.iter().any(|e| e.field == "search_surface"));
    assert!(errs.iter().any(|e| e.field == "time_budget_secs"));
}

// =======================================================================
// Group D — Block structural validity
// =======================================================================

#[test]
fn d_valid_block_passes_validation() {
    let b = valid_block();
    assert!(
        validate_block_structure(&b).is_ok(),
        "valid block fixture should pass structural validation"
    );
}

#[test]
fn d_block_missing_evidence_fails() {
    let b = invalid_block_missing_evidence();
    let errs = validate_block_structure(&b).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "evidence_bundle_hash"));
}

#[test]
fn d_block_missing_child_state_fails() {
    let b = invalid_block_missing_child_state();
    let errs = validate_block_structure(&b).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "child_state_ref"));
}

#[test]
fn d_block_missing_diff_fails() {
    let b = invalid_block_missing_diff();
    let errs = validate_block_structure(&b).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "diff_ref"));
}

#[test]
fn d_block_nan_delta_fails() {
    let b = invalid_block_nan_delta();
    let errs = validate_block_structure(&b).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "claimed_metric_delta"));
}

#[test]
fn d_block_inf_delta_fails() {
    let b = invalid_block_inf_delta();
    let errs = validate_block_structure(&b).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "claimed_metric_delta"));
}

/// A block references its parent (the genesis block) via explicit
/// conversion. Verify the parent_id field is populated correctly.
#[test]
fn d_block_parent_references_genesis() {
    let b = valid_block();
    let expected_parent = test_genesis_block_id(1).as_block_id();
    assert_eq!(b.parent_id, expected_parent);
}

// =======================================================================
// Group E — ValidationAttestation structural validity
// =======================================================================

#[test]
fn e_valid_attestation_passes_validation() {
    let a = valid_attestation();
    assert!(validate_attestation_structure(&a).is_ok());
}

#[test]
fn e_attestation_missing_evidence_fails() {
    let a = invalid_attestation_missing_evidence();
    let errs = validate_attestation_structure(&a).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "replay_evidence_ref"));
}

/// Attestation includes all expected fields and they are accessible.
#[test]
fn e_attestation_field_integrity() {
    let a = valid_attestation();
    assert_eq!(a.block_id, test_block_id(2));
    assert_eq!(a.validator, test_validator_id(1));
    assert_eq!(a.vote, ValidatorVote::Pass);
    assert_ne!(a.replay_evidence_ref, ArtifactHash::ZERO);
    assert!(a.timestamp > 0);
}

/// All ValidatorVote variants can be used in an attestation.
#[test]
fn e_attestation_all_vote_types() {
    let votes = [
        ValidatorVote::Pass,
        ValidatorVote::Fail,
        ValidatorVote::Inconclusive,
        ValidatorVote::FraudSuspected,
    ];
    for vote in &votes {
        let mut a = valid_attestation();
        a.vote = *vote;
        assert!(validate_attestation_structure(&a).is_ok());
        enum_roundtrip(&a);
    }
}

// =======================================================================
// Group F — ChallengeRecord structural validity
// =======================================================================

#[test]
fn f_valid_challenge_passes_validation() {
    let c = valid_challenge();
    assert!(validate_challenge_structure(&c).is_ok());
}

#[test]
fn f_challenge_missing_evidence_fails() {
    let c = invalid_challenge_missing_evidence();
    let errs = validate_challenge_structure(&c).unwrap_err();
    assert!(errs.iter().any(|e| e.field == "evidence_ref"));
}

/// All ChallengeType variants can be used in a challenge record.
#[test]
fn f_challenge_all_types() {
    let types = [
        ChallengeType::BlockReplay,
        ChallengeType::AttestationFraud,
        ChallengeType::Attribution,
        ChallengeType::Dominance,
        ChallengeType::MetricAdequacy,
    ];
    for ct in &types {
        let mut c = valid_challenge();
        c.challenge_type = *ct;
        assert!(validate_challenge_structure(&c).is_ok());
        enum_roundtrip(&c);
    }
}

/// Challenge field integrity: all required fields are populated.
#[test]
fn f_challenge_field_integrity() {
    let c = valid_challenge();
    assert_ne!(c.id, ChallengeId::ZERO);
    assert_ne!(c.target_block_id, BlockId::ZERO);
    assert_ne!(c.challenger, ParticipantId::ZERO);
    assert_ne!(c.evidence_ref, ArtifactHash::ZERO);
    assert_eq!(c.status, ChallengeStatus::Open);
}

// =======================================================================
// Group G — Canonical fixture round-trips (fixture integrity)
// =======================================================================

#[test]
fn g_fixture_valid_genesis_round_trips() {
    let g = valid_genesis_block();
    let json = serde_json::to_string(&g).unwrap();
    let recovered: GenesisBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(g, recovered);
}

#[test]
fn g_fixture_valid_block_round_trips() {
    let b = valid_block();
    let json = serde_json::to_string(&b).unwrap();
    let recovered: Block = serde_json::from_str(&json).unwrap();
    assert_eq!(b, recovered);
}

#[test]
fn g_fixture_valid_attestation_round_trips() {
    let a = valid_attestation();
    let json = serde_json::to_string(&a).unwrap();
    let recovered: ValidationAttestation = serde_json::from_str(&json).unwrap();
    assert_eq!(a, recovered);
}

#[test]
fn g_fixture_valid_challenge_round_trips() {
    let c = valid_challenge();
    let json = serde_json::to_string(&c).unwrap();
    let recovered: ChallengeRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(c, recovered);
}

#[test]
fn g_fixture_valid_frontier_round_trips() {
    let f = valid_frontier_state();
    let json = serde_json::to_string(&f).unwrap();
    let recovered: CanonicalFrontierState = serde_json::from_str(&json).unwrap();
    assert_eq!(f, recovered);
}

// =======================================================================
// Group H — Serialization determinism
// =======================================================================

/// Helper: serialize twice and verify identical output.
fn assert_deterministic_serialization<T: serde::Serialize>(val: &T, label: &str) {
    let json1 = serde_json::to_string(val).unwrap();
    let json2 = serde_json::to_string(val).unwrap();
    assert_eq!(json1, json2, "non-deterministic serialization for {}", label);
}

/// Helper: serialize, deserialize, re-serialize, and verify stability.
fn assert_stable_roundtrip<T>(val: &T, label: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json1 = serde_json::to_string(val).unwrap();
    let recovered: T = serde_json::from_str(&json1).unwrap();
    assert_eq!(val, &recovered, "roundtrip equality failed for {}", label);
    let json2 = serde_json::to_string(&recovered).unwrap();
    assert_eq!(
        json1, json2,
        "re-serialization drift detected for {}",
        label
    );
}

#[test]
fn h_genesis_block_serialization_determinism() {
    let g = valid_genesis_block();
    assert_deterministic_serialization(&g, "GenesisBlock");
    assert_stable_roundtrip(&g, "GenesisBlock");
}

#[test]
fn h_block_serialization_determinism() {
    let b = valid_block();
    assert_deterministic_serialization(&b, "Block");
    assert_stable_roundtrip(&b, "Block");
}

#[test]
fn h_attestation_serialization_determinism() {
    let a = valid_attestation();
    assert_deterministic_serialization(&a, "ValidationAttestation");
    assert_stable_roundtrip(&a, "ValidationAttestation");
}

#[test]
fn h_challenge_serialization_determinism() {
    let c = valid_challenge();
    assert_deterministic_serialization(&c, "ChallengeRecord");
    assert_stable_roundtrip(&c, "ChallengeRecord");
}

#[test]
fn h_frontier_state_serialization_determinism() {
    let f = valid_frontier_state();
    assert_deterministic_serialization(&f, "CanonicalFrontierState");
    assert_stable_roundtrip(&f, "CanonicalFrontierState");
}

#[test]
fn h_materialized_state_serialization_determinism() {
    let ms = MaterializedState {
        id: MaterializedStateId::from_bytes([1u8; 32]),
        domain_id: test_domain_id(1),
        root_tree_hash: test_artifact_hash(1),
        resolved_dependency_manifest_hash: test_artifact_hash(2),
        resolved_config_hash: test_artifact_hash(3),
        environment_manifest_hash: test_artifact_hash(4),
        evaluation_manifest_hash: test_artifact_hash(5),
        materialized_from_block_id: test_block_id(1),
        timestamp: 1700000000,
    };
    assert_deterministic_serialization(&ms, "MaterializedState");
    assert_stable_roundtrip(&ms, "MaterializedState");
}

#[test]
fn h_codebase_state_ref_all_variants_determinism() {
    let refs = [
        CodebaseStateRef::LatestFrontier {
            domain_id: test_domain_id(1),
        },
        CodebaseStateRef::Historical {
            materialized_state_id: MaterializedStateId::from_bytes([2u8; 32]),
        },
        CodebaseStateRef::AtBlock {
            block_id: test_block_id(3),
        },
    ];
    for (i, r) in refs.iter().enumerate() {
        let label = format!("CodebaseStateRef variant {}", i);
        assert_deterministic_serialization(r, &label);
        assert_stable_roundtrip(r, &label);
    }
}

#[test]
fn h_problem_domain_serialization_determinism() {
    let d = ProblemDomain {
        id: test_domain_id(1),
        name: "cifar10-recipe".to_string(),
        domain_type: DomainType::Model,
        parent_domain_id: None,
        spec_id: DomainSpecId::from_bytes([5u8; 32]),
    };
    assert_deterministic_serialization(&d, "ProblemDomain");
    assert_stable_roundtrip(&d, "ProblemDomain");
}

#[test]
fn h_domain_spec_serialization_determinism() {
    let spec = DomainSpec {
        id: DomainSpecId::from_bytes([1u8; 32]),
        domain_id: test_domain_id(1),
        base_codebase_ref: test_artifact_hash(1),
        primary_metric: "test_accuracy".to_string(),
        metric_direction: MetricDirection::HigherBetter,
        secondary_metrics: vec!["train_loss".to_string()],
        search_surface: vec!["train.py".to_string()],
        frozen_surface: vec!["eval/".to_string()],
        artifact_schema_ref: test_artifact_hash(2),
        hardware_class: "RTX 4090".to_string(),
        materialization_policy: MaterializationPolicyKind::OnDominance,
    };
    assert_deterministic_serialization(&spec, "DomainSpec");
    assert_stable_roundtrip(&spec, "DomainSpec");
}

/// Optional fields (None vs Some) produce distinct, stable serialization.
#[test]
fn h_optional_fields_stable() {
    // ProblemDomain with and without parent
    let d_no_parent = ProblemDomain {
        id: test_domain_id(1),
        name: "root".to_string(),
        domain_type: DomainType::Root,
        parent_domain_id: None,
        spec_id: DomainSpecId::from_bytes([1u8; 32]),
    };
    let d_with_parent = ProblemDomain {
        id: test_domain_id(2),
        name: "child".to_string(),
        domain_type: DomainType::Subsystem,
        parent_domain_id: Some(test_domain_id(1)),
        spec_id: DomainSpecId::from_bytes([2u8; 32]),
    };

    let json_no = serde_json::to_string(&d_no_parent).unwrap();
    let json_with = serde_json::to_string(&d_with_parent).unwrap();

    // They must be different
    assert_ne!(json_no, json_with);

    // Both must round-trip stably
    assert_stable_roundtrip(&d_no_parent, "ProblemDomain(no parent)");
    assert_stable_roundtrip(&d_with_parent, "ProblemDomain(with parent)");

    // Verify None serializes as null
    assert!(json_no.contains("null"), "None should serialize as null");
}

/// DatasetSplits with and without test partition.
#[test]
fn h_dataset_splits_optional_test_stable() {
    let with_test = DatasetSplits {
        training: test_artifact_hash(1),
        validation: test_artifact_hash(2),
        test: Some(test_artifact_hash(3)),
    };
    let without_test = DatasetSplits {
        training: test_artifact_hash(1),
        validation: test_artifact_hash(2),
        test: None,
    };

    assert_stable_roundtrip(&with_test, "DatasetSplits(with test)");
    assert_stable_roundtrip(&without_test, "DatasetSplits(without test)");
    assert_ne!(
        serde_json::to_string(&with_test).unwrap(),
        serde_json::to_string(&without_test).unwrap(),
    );
}

/// Escrow record serialization determinism.
#[test]
fn h_escrow_record_serialization_determinism() {
    let e = EscrowRecord {
        id: EscrowId::from_bytes([1u8; 32]),
        block_id: test_block_id(1),
        beneficiary: test_participant_id(1),
        amount: 500,
        status: EscrowStatus::Held,
        created_epoch: EpochId(1),
        release_epoch: EpochId(10),
    };
    assert_deterministic_serialization(&e, "EscrowRecord");
    assert_stable_roundtrip(&e, "EscrowRecord");
}

/// ForkFamily serialization determinism.
#[test]
fn h_fork_family_serialization_determinism() {
    let ff = ForkFamily {
        id: ForkFamilyId::from_bytes([1u8; 32]),
        domain_id: test_domain_id(1),
        track_tree_id: TrackTreeId::from_bytes([2u8; 32]),
        common_ancestor_id: test_block_id(1),
        branch_tips: vec![test_block_id(2), test_block_id(3)],
        dominant_branch_tip: None,
    };
    assert_deterministic_serialization(&ff, "ForkFamily");
    assert_stable_roundtrip(&ff, "ForkFamily");
}

/// TrackTree serialization determinism.
#[test]
fn h_track_tree_serialization_determinism() {
    let gid = test_genesis_block_id(1);
    let tt = TrackTree {
        id: gid.as_track_tree_id(),
        domain_id: test_domain_id(1),
        genesis_block_id: gid,
        fork_families: vec![ForkFamilyId::from_bytes([1u8; 32])],
        canonical_frontier_block_id: Some(test_block_id(5)),
    };
    assert_deterministic_serialization(&tt, "TrackTree");
    assert_stable_roundtrip(&tt, "TrackTree");
}

/// EvidenceBundle serialization determinism.
#[test]
fn h_evidence_bundle_serialization_determinism() {
    let eb = EvidenceBundle {
        block_id: test_block_id(1),
        diff_ref: test_artifact_hash(1),
        config_ref: test_artifact_hash(2),
        environment_manifest_ref: test_artifact_hash(3),
        dataset_refs: vec![test_artifact_hash(4), test_artifact_hash(5)],
        evaluation_procedure_ref: test_artifact_hash(6),
        training_log_ref: test_artifact_hash(7),
        metric_output_ref: test_artifact_hash(8),
    };
    assert_deterministic_serialization(&eb, "EvidenceBundle");
    assert_stable_roundtrip(&eb, "EvidenceBundle");
}

/// Policy types serialization determinism.
#[test]
fn h_metric_integrity_policy_serialization_determinism() {
    let p = MetricIntegrityPolicy {
        track_id: test_genesis_block_id(1),
        metric_id: "test_accuracy".to_string(),
        metric_direction: MetricDirection::HigherBetter,
        evaluation_harness_ref: test_artifact_hash(1),
        tolerance: 0.005,
        max_replay_budget_secs: 3600,
    };
    assert_deterministic_serialization(&p, "MetricIntegrityPolicy");
    assert_stable_roundtrip(&p, "MetricIntegrityPolicy");
}

#[test]
fn h_dataset_integrity_policy_serialization_determinism() {
    let p = DatasetIntegrityPolicy {
        track_id: test_genesis_block_id(1),
        canonical_dataset_ref: test_artifact_hash(1),
        dataset_hash: test_artifact_hash(2),
        splits: DatasetSplits {
            training: test_artifact_hash(3),
            validation: test_artifact_hash(4),
            test: None,
        },
        availability_requirement: "publicly downloadable".to_string(),
        license_declaration: "CC-BY-4.0".to_string(),
    };
    assert_deterministic_serialization(&p, "DatasetIntegrityPolicy");
    assert_stable_roundtrip(&p, "DatasetIntegrityPolicy");
}

// =======================================================================
// Group I — Invariant helper coverage
// =======================================================================
//
// Groups C-F exercise the validate_*_structure() helpers directly.
// These additional tests verify edge cases and helper behavior.

/// StructuralError displays field and reason clearly.
#[test]
fn i_structural_error_display() {
    let err = StructuralError {
        field: "metric_id",
        reason: "must not be empty",
    };
    let displayed = format!("{}", err);
    assert_eq!(displayed, "metric_id: must not be empty");
}

/// Validation helpers return Ok for all valid fixtures.
#[test]
fn i_all_valid_fixtures_pass() {
    assert!(validate_genesis_block_structure(&valid_genesis_block()).is_ok());
    assert!(validate_block_structure(&valid_block()).is_ok());
    assert!(validate_attestation_structure(&valid_attestation()).is_ok());
    assert!(validate_challenge_structure(&valid_challenge()).is_ok());
}

/// Validation helpers return Err for all invalid fixtures.
#[test]
fn i_all_invalid_fixtures_fail() {
    // Genesis variants
    assert!(validate_genesis_block_structure(&invalid_genesis_missing_metric_id()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_missing_dataset_hash()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_empty_search_surface()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_empty_frozen_surface()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_missing_research_target()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_nan_seed_score()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_inf_seed_score()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_zero_time_budget()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_missing_hardware_class()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_missing_eval_harness()).is_err());
    assert!(validate_genesis_block_structure(&invalid_genesis_missing_seed_recipe()).is_err());

    // Block variants
    assert!(validate_block_structure(&invalid_block_missing_evidence()).is_err());
    assert!(validate_block_structure(&invalid_block_missing_child_state()).is_err());
    assert!(validate_block_structure(&invalid_block_missing_diff()).is_err());
    assert!(validate_block_structure(&invalid_block_nan_delta()).is_err());
    assert!(validate_block_structure(&invalid_block_inf_delta()).is_err());

    // Attestation variants
    assert!(validate_attestation_structure(&invalid_attestation_missing_evidence()).is_err());

    // Challenge variants
    assert!(validate_challenge_structure(&invalid_challenge_missing_evidence()).is_err());
}

/// Each invalid fixture produces exactly one error (single-fault isolation).
#[test]
fn i_invalid_fixtures_single_fault() {
    // Each fixture violates exactly one invariant, so the error vec
    // should contain exactly one entry.
    let cases: Vec<(&str, Result<(), Vec<StructuralError>>)> = vec![
        ("genesis/metric_id", validate_genesis_block_structure(&invalid_genesis_missing_metric_id())),
        ("genesis/dataset_hash", validate_genesis_block_structure(&invalid_genesis_missing_dataset_hash())),
        ("genesis/search_surface", validate_genesis_block_structure(&invalid_genesis_empty_search_surface())),
        ("genesis/frozen_surface", validate_genesis_block_structure(&invalid_genesis_empty_frozen_surface())),
        ("genesis/research_target", validate_genesis_block_structure(&invalid_genesis_missing_research_target())),
        ("genesis/nan_score", validate_genesis_block_structure(&invalid_genesis_nan_seed_score())),
        ("genesis/inf_score", validate_genesis_block_structure(&invalid_genesis_inf_seed_score())),
        ("genesis/time_budget", validate_genesis_block_structure(&invalid_genesis_zero_time_budget())),
        ("genesis/hardware", validate_genesis_block_structure(&invalid_genesis_missing_hardware_class())),
        ("genesis/eval_harness", validate_genesis_block_structure(&invalid_genesis_missing_eval_harness())),
        ("genesis/seed_recipe", validate_genesis_block_structure(&invalid_genesis_missing_seed_recipe())),
        ("block/evidence", validate_block_structure(&invalid_block_missing_evidence())),
        ("block/child_state", validate_block_structure(&invalid_block_missing_child_state())),
        ("block/diff", validate_block_structure(&invalid_block_missing_diff())),
        ("block/nan_delta", validate_block_structure(&invalid_block_nan_delta())),
        ("block/inf_delta", validate_block_structure(&invalid_block_inf_delta())),
        ("attestation/evidence", validate_attestation_structure(&invalid_attestation_missing_evidence())),
        ("challenge/evidence", validate_challenge_structure(&invalid_challenge_missing_evidence())),
    ];

    for (label, result) in cases {
        let errs = result.unwrap_err();
        assert_eq!(
            errs.len(),
            1,
            "{}: expected exactly 1 error, got {}: {:?}",
            label,
            errs.len(),
            errs
        );
    }
}

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

//! Unified transaction application (Milestone E2/E3).
//!
//! Every state mutation on a networked node — whether submitted over HTTP
//! or replayed from the ordering log — flows through [`apply_tx`]: one
//! dispatcher keyed by transaction kind, with the same signature policy
//! the CLI enforces. This is what makes the ordering log a replication
//! log: applying the same entries in the same order to the same initial
//! state must produce the same final state on every node.

use arc_protocol_types::*;
use arc_domain_engine::genesis::SeedValidationRecord;
use arc_protocol_rules::validator::ValidatorPool;
use arc_simulator::state::SimulatorState;

fn jstr<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

fn ju(v: &serde_json::Value, k: &str) -> u64 {
    v.get(k).and_then(|x| x.as_u64()).unwrap_or(0)
}

fn jf(v: &serde_json::Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}

fn parse_hex32(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("expected 64 hex characters, got {}", hex.len()));
    }
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("invalid hex: {}", e))?;
    }
    Ok(bytes)
}

fn extract_signature(v: &serde_json::Value) -> Result<Option<Vec<u8>>, String> {
    match v.get("signature").and_then(|s| s.as_str()) {
        None => Ok(None),
        Some(hex) => {
            if hex.len() != 128 {
                return Err("signature must be 128 hex characters".to_string());
            }
            let mut bytes = vec![0u8; 64];
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                    .map_err(|e| format!("invalid signature hex: {}", e))?;
            }
            Ok(Some(bytes))
        }
    }
}

fn check_signature(
    sim: &SimulatorState,
    actor_hex: &str,
    message: &[u8],
    payload: &serde_json::Value,
) -> Result<(), String> {
    match extract_signature(payload)? {
        None if sim.require_signatures => Err(format!(
            "state requires signatures: unsigned submission from {}",
            actor_hex
        )),
        None => Ok(()),
        Some(sig) => {
            let pk = parse_hex32(actor_hex)?;
            arc_identity::verify(&pk, message, &sig)
                .map_err(|e| format!("signature rejected for {}: {}", actor_hex, e))
        }
    }
}

fn from_payload<T: serde::de::DeserializeOwned>(
    v: &serde_json::Value,
) -> Result<T, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("cannot parse payload: {}", e))
}

/// Apply one transaction to the state.
///
/// `kind` matches the CLI command names. Payload conventions:
/// - Object-bearing kinds take the same JSON as their CLI file input
///   (with an optional sibling `signature` field).
/// - ID-argument kinds take `{"id": "<hex>"}`.
/// - `record-seed-validation` takes the record plus `"genesis_id"`.
/// - `advance-epoch` takes `{}`.
///
/// Returns a JSON result value on success.
pub fn apply_tx(
    sim: &mut SimulatorState,
    kind: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let id32 = |field: &str| -> Result<[u8; 32], String> { parse_hex32(jstr(payload, field)) };

    match kind {
        "submit-genesis" => {
            let message = arc_identity::genesis_message(
                jstr(payload, "id"),
                jstr(payload, "proposer"),
                ju(payload, "timestamp"),
            );
            check_signature(sim, jstr(payload, "proposer"), &message, payload)?;
            let genesis: GenesisBlock = from_payload(payload)?;
            let id = sim.submit_genesis(genesis)?;
            Ok(serde_json::json!({ "genesis_id": id }))
        }
        "evaluate-conformance" => {
            let id = GenesisBlockId::from_bytes(id32("id")?);
            sim.evaluate_conformance(&id)?;
            Ok(serde_json::json!({ "status": "conformance_passed" }))
        }
        "record-seed-validation" => {
            let genesis_hex = jstr(payload, "genesis_id").to_string();
            let id = GenesisBlockId::from_bytes(parse_hex32(&genesis_hex)?);
            let message = arc_identity::seed_validation_message(
                &genesis_hex,
                jstr(payload, "validator"),
                jstr(payload, "vote"),
                jf(payload, "observed_score"),
                ju(payload, "timestamp"),
            );
            check_signature(sim, jstr(payload, "validator"), &message, payload)?;
            let record: SeedValidationRecord = from_payload(payload)?;
            sim.record_seed_validation(&id, record)?;
            Ok(serde_json::json!({ "status": "seed_validation_recorded" }))
        }
        "finalize-activation" => {
            let id = GenesisBlockId::from_bytes(id32("id")?);
            let activated = sim.finalize_activation(&id)?;
            Ok(serde_json::json!({
                "status": "domain_activated",
                "domain_id": activated.domain.id,
            }))
        }
        "register-validators" => {
            let pool: ValidatorPool = from_payload(payload)?;
            let count = pool.validators.len();
            sim.register_validator_pool(pool);
            Ok(serde_json::json!({
                "status": "validators_registered",
                "validator_count": count,
            }))
        }
        "submit-block" => {
            let message = arc_identity::block_message(
                jstr(payload, "id"),
                jstr(payload, "domain_id"),
                jstr(payload, "parent_id"),
                jstr(payload, "proposer"),
                jstr(payload, "child_state_ref"),
                jstr(payload, "diff_ref"),
                jf(payload, "claimed_metric_delta").unwrap_or(0.0),
                jstr(payload, "evidence_bundle_hash"),
                ju(payload, "fee"),
                ju(payload, "bond"),
                ju(payload, "epoch_id"),
                ju(payload, "timestamp"),
            );
            check_signature(sim, jstr(payload, "proposer"), &message, payload)?;
            let block: Block = from_payload(payload)?;
            let block_id = sim.submit_block(block)?;
            Ok(serde_json::json!({ "block_id": block_id }))
        }
        "assign-validators" => {
            let id = BlockId::from_bytes(id32("id")?);
            let assigned = sim.assign_validators(&id)?;
            Ok(serde_json::json!({ "assigned_validators": assigned }))
        }
        "submit-attestation" => {
            let message = arc_identity::attestation_message(
                jstr(payload, "block_id"),
                jstr(payload, "validator"),
                jstr(payload, "vote"),
                jf(payload, "observed_delta"),
                jstr(payload, "replay_evidence_ref"),
                ju(payload, "timestamp"),
            );
            check_signature(sim, jstr(payload, "validator"), &message, payload)?;
            let attestation: ValidationAttestation = from_payload(payload)?;
            sim.record_attestation(attestation)?;
            Ok(serde_json::json!({ "status": "attestation_recorded" }))
        }
        "evaluate-block" => {
            let id = BlockId::from_bytes(id32("id")?);
            let outcome = sim.evaluate_block(&id)?;
            Ok(serde_json::json!({ "outcome": format!("{:?}", outcome) }))
        }
        "close-challenge-window" => {
            let id = BlockId::from_bytes(id32("id")?);
            sim.close_challenge_window(&id)?;
            Ok(serde_json::json!({ "status": "challenge_window_closed" }))
        }
        "settle-block" => {
            let id = BlockId::from_bytes(id32("id")?);
            sim.settle_block(&id)?;
            Ok(serde_json::json!({ "status": "block_settled" }))
        }
        "finalize-block" => {
            let id = BlockId::from_bytes(id32("id")?);
            sim.finalize_block(&id)?;
            Ok(serde_json::json!({ "status": "block_finalized" }))
        }
        "open-challenge" => {
            let target_hex = payload
                .get("target")
                .and_then(|t| t.as_object())
                .and_then(|o| o.values().next())
                .and_then(|inner| inner.get("block_id"))
                .and_then(|b| b.as_str())
                .unwrap_or("");
            let message = arc_identity::challenge_message(
                jstr(payload, "challenge_id"),
                jstr(payload, "challenge_type"),
                target_hex,
                jstr(payload, "challenger"),
                ju(payload, "bond"),
                jstr(payload, "evidence_ref"),
            );
            check_signature(sim, jstr(payload, "challenger"), &message, payload)?;

            let challenge_id = ChallengeId::from_bytes(parse_hex32(jstr(payload, "challenge_id"))?);
            let challenge_type: ChallengeType =
                from_payload(payload.get("challenge_type").unwrap_or(&serde_json::Value::Null))
                    .map_err(|e| format!("challenge_type: {}", e))?;
            let target: ChallengeTarget =
                from_payload(payload.get("target").unwrap_or(&serde_json::Value::Null))
                    .map_err(|e| format!("target: {}", e))?;
            let challenger = ParticipantId::from_bytes(parse_hex32(jstr(payload, "challenger"))?);
            let bond = TokenAmount::new(ju(payload, "bond"));
            let evidence_ref = ArtifactHash::from_bytes(parse_hex32(jstr(payload, "evidence_ref"))?);
            let id = sim.open_challenge(
                challenge_id,
                challenge_type,
                target,
                challenger,
                bond,
                evidence_ref,
            )?;
            Ok(serde_json::json!({ "challenge_id": id }))
        }
        "begin-review" => {
            let id = ChallengeId::from_bytes(id32("id")?);
            sim.begin_challenge_review(&id)?;
            Ok(serde_json::json!({ "status": "challenge_under_review" }))
        }
        "uphold-challenge" => {
            let id = ChallengeId::from_bytes(id32("id")?);
            sim.uphold_challenge(&id)?;
            let mut result = serde_json::json!({ "status": "challenge_upheld" });
            if let Some(dist) = sim.slash_distribution(&id) {
                result["slash_distribution"] =
                    serde_json::to_value(dist).map_err(|e| e.to_string())?;
            }
            Ok(result)
        }
        "reject-challenge" => {
            let id = ChallengeId::from_bytes(id32("id")?);
            sim.reject_challenge(&id)?;
            Ok(serde_json::json!({ "status": "challenge_rejected" }))
        }
        "expire-challenge" => {
            let id = ChallengeId::from_bytes(id32("id")?);
            sim.expire_challenge(&id)?;
            Ok(serde_json::json!({ "status": "challenge_expired" }))
        }
        "advance-epoch" => {
            sim.advance_epoch();
            Ok(serde_json::json!({ "epoch": sim.current_epoch.0 }))
        }
        other => Err(format!("unknown transaction kind: {}", other)),
    }
}

/// Canonical state hash for cross-node comparison: BLAKE3 of the state's
/// canonical JSON (serde_json::Value objects serialize with sorted keys).
pub fn state_hash(sim: &SimulatorState) -> String {
    let value = serde_json::to_value(sim).expect("state serialization failed");
    let canonical = serde_json::to_string(&value).expect("canonical serialization failed");
    blake3::hash(canonical.as_bytes()).to_hex().to_string()
}

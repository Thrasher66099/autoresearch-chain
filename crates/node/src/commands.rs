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

//! Write-action CLI command implementations.
//!
//! Each command follows the same pattern:
//! 1. Load state from the state file
//! 2. Parse command-specific input (JSON file or hex ID)
//! 3. Delegate to the corresponding `SimulatorState` method
//! 4. Save state back to the state file
//! 5. Print JSON result to stdout
//!
//! All protocol validation happens inside `SimulatorState`. These commands
//! are thin wrappers — no convenience shortcuts that bypass validation.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use arc_protocol_types::*;
use arc_domain_engine::genesis::SeedValidationRecord;
use arc_protocol_rules::validator::ValidatorPool;
use arc_simulator::state::SimulatorState;

use crate::persistence;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

/// Load and deserialize a JSON file.
fn load_json_file<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path, e))?;
    serde_json::from_str(&contents)
        .map_err(|e| format!("cannot parse {}: {}", path, e))
}

/// Parse a 64-character hex string into a 32-byte array.
fn parse_hex_bytes(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!(
            "expected 64 hex characters, got {} (\"{}\")",
            hex.len(),
            hex
        ));
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("invalid hex: {}", e))?;
    }
    Ok(bytes)
}

fn parse_block_id(hex: &str) -> Result<BlockId, String> {
    Ok(BlockId::from_bytes(parse_hex_bytes(hex)?))
}

fn parse_genesis_block_id(hex: &str) -> Result<GenesisBlockId, String> {
    Ok(GenesisBlockId::from_bytes(parse_hex_bytes(hex)?))
}

fn parse_challenge_id(hex: &str) -> Result<ChallengeId, String> {
    Ok(ChallengeId::from_bytes(parse_hex_bytes(hex)?))
}

/// Load a payload JSON file as a raw value plus its optional detached
/// `signature` field (128 hex chars = 64 bytes). The signature is a
/// sibling of the payload fields and ignored by typed deserialization.
fn load_signed_json(path: &str) -> Result<(serde_json::Value, Option<Vec<u8>>), String> {
    let value: serde_json::Value = load_json_file(path)?;
    let sig = match value.get("signature").and_then(|s| s.as_str()) {
        None => None,
        Some(hex) => {
            if hex.len() != 128 {
                return Err(format!(
                    "signature must be 128 hex characters, got {}",
                    hex.len()
                ));
            }
            let mut bytes = vec![0u8; 64];
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                    .map_err(|e| format!("invalid signature hex: {}", e))?;
            }
            Some(bytes)
        }
    };
    Ok((value, sig))
}

/// Enforce the signature policy: when the state requires signatures the
/// payload must carry one; any signature present is verified against the
/// actor's ID (which is the Ed25519 public key).
fn check_signature(
    sim: &SimulatorState,
    actor_hex: &str,
    message: &[u8],
    signature: &Option<Vec<u8>>,
) -> Result<(), String> {
    match signature {
        None if sim.require_signatures => Err(format!(
            "state requires signatures: unsigned submission from {}",
            actor_hex
        )),
        None => Ok(()),
        Some(sig) => {
            let pk = parse_hex_bytes(actor_hex)?;
            arc_identity::verify(&pk, message, sig)
                .map_err(|e| format!("signature rejected for {}: {}", actor_hex, e))
        }
    }
}

fn jstr<'a>(v: &'a serde_json::Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}

fn ju(v: &serde_json::Value, k: &str) -> u64 {
    v.get(k).and_then(|x| x.as_u64()).unwrap_or(0)
}

fn jf(v: &serde_json::Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}

/// Load state, apply a mutation, save state, and return a JSON result.
fn load_mutate_save<F, R>(state_path: &Path, action: F) -> Result<R, String>
where
    F: FnOnce(&mut SimulatorState) -> Result<R, String>,
{
    let mut state = persistence::load_state(state_path)
        .map_err(|e| format!("{}", e))?;
    let result = action(&mut state)?;
    persistence::save_state(&state, state_path)
        .map_err(|e| format!("{}", e))?;
    Ok(result)
}

/// Print a serializable value as JSON to stdout.
fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("serialization failed")
    );
}

// -----------------------------------------------------------------------
// Command implementations
// -----------------------------------------------------------------------

/// `submit-genesis <json-file>`
pub fn cmd_submit_genesis(state_path: &Path, args: &[String]) {
    let json_file = args.first().unwrap_or_else(|| {
        eprintln!("error: submit-genesis requires a JSON file argument");
        std::process::exit(1);
    });

    let (raw, sig) = match load_signed_json(json_file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    let genesis: GenesisBlock = match serde_json::from_value(raw.clone()) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: cannot parse {}: {}", json_file, e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        let message = arc_identity::genesis_message(
            jstr(&raw, "id"),
            jstr(&raw, "proposer"),
            ju(&raw, "timestamp"),
        );
        check_signature(sim, jstr(&raw, "proposer"), &message, &sig)?;
        let id = sim.submit_genesis(genesis)?;
        Ok(serde_json::json!({ "genesis_id": id }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `evaluate-conformance <genesis-id>`
pub fn cmd_evaluate_conformance(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: evaluate-conformance requires a genesis ID (hex)");
        std::process::exit(1);
    });

    let genesis_id = match parse_genesis_block_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.evaluate_conformance(&genesis_id)?;
        Ok(serde_json::json!({ "status": "conformance_passed", "genesis_id": genesis_id }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `record-seed-validation <genesis-id> <json-file>`
pub fn cmd_record_seed_validation(state_path: &Path, args: &[String]) {
    if args.len() < 2 {
        eprintln!("error: record-seed-validation requires <genesis-id> <json-file>");
        std::process::exit(1);
    }

    let genesis_id = match parse_genesis_block_id(&args[0]) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let (raw, sig) = match load_signed_json(&args[1]) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    let record: SeedValidationRecord = match serde_json::from_value(raw.clone()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: cannot parse {}: {}", &args[1], e);
            std::process::exit(1);
        }
    };

    let genesis_hex = args[0].clone();
    match load_mutate_save(state_path, |sim| {
        let message = arc_identity::seed_validation_message(
            &genesis_hex,
            jstr(&raw, "validator"),
            jstr(&raw, "vote"),
            jf(&raw, "observed_score"),
            ju(&raw, "timestamp"),
        );
        check_signature(sim, jstr(&raw, "validator"), &message, &sig)?;
        sim.record_seed_validation(&genesis_id, record)?;
        Ok(serde_json::json!({ "status": "seed_validation_recorded", "genesis_id": genesis_id }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `finalize-activation <genesis-id>`
pub fn cmd_finalize_activation(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: finalize-activation requires a genesis ID (hex)");
        std::process::exit(1);
    });

    let genesis_id = match parse_genesis_block_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        let activated = sim.finalize_activation(&genesis_id)?;
        Ok(serde_json::json!({
            "status": "domain_activated",
            "domain_id": activated.domain.id,
            "genesis_id": genesis_id,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `register-validators <json-file>`
pub fn cmd_register_validators(state_path: &Path, args: &[String]) {
    let json_file = args.first().unwrap_or_else(|| {
        eprintln!("error: register-validators requires a JSON file argument");
        std::process::exit(1);
    });

    let pool: ValidatorPool = match load_json_file(json_file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let domain_id = pool.domain_id;
    let count = pool.validators.len();

    match load_mutate_save(state_path, |sim| {
        sim.register_validator_pool(pool);
        Ok(serde_json::json!({
            "status": "validators_registered",
            "domain_id": domain_id,
            "validator_count": count,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `submit-block <json-file>`
pub fn cmd_submit_block(state_path: &Path, args: &[String]) {
    let json_file = args.first().unwrap_or_else(|| {
        eprintln!("error: submit-block requires a JSON file argument");
        std::process::exit(1);
    });

    let (raw, sig) = match load_signed_json(json_file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    let block: Block = match serde_json::from_value(raw.clone()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot parse {}: {}", json_file, e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        let message = arc_identity::block_message(
            jstr(&raw, "id"),
            jstr(&raw, "domain_id"),
            jstr(&raw, "parent_id"),
            jstr(&raw, "proposer"),
            jstr(&raw, "child_state_ref"),
            jstr(&raw, "diff_ref"),
            jf(&raw, "claimed_metric_delta").unwrap_or(0.0),
            jstr(&raw, "evidence_bundle_hash"),
            ju(&raw, "fee"),
            ju(&raw, "bond"),
            ju(&raw, "epoch_id"),
            ju(&raw, "timestamp"),
        );
        check_signature(sim, jstr(&raw, "proposer"), &message, &sig)?;
        let block_id = sim.submit_block(block)?;
        Ok(serde_json::json!({ "block_id": block_id }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `assign-validators <block-id>`
pub fn cmd_assign_validators(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: assign-validators requires a block ID (hex)");
        std::process::exit(1);
    });

    let block_id = match parse_block_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        let assigned = sim.assign_validators(&block_id)?;
        Ok(serde_json::json!({
            "block_id": block_id,
            "assigned_validators": assigned,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `submit-attestation <json-file>`
pub fn cmd_submit_attestation(state_path: &Path, args: &[String]) {
    let json_file = args.first().unwrap_or_else(|| {
        eprintln!("error: submit-attestation requires a JSON file argument");
        std::process::exit(1);
    });

    let (raw, sig) = match load_signed_json(json_file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    let attestation: ValidationAttestation = match serde_json::from_value(raw.clone()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: cannot parse {}: {}", json_file, e);
            std::process::exit(1);
        }
    };

    let block_id = attestation.block_id;

    match load_mutate_save(state_path, |sim| {
        let message = arc_identity::attestation_message(
            jstr(&raw, "block_id"),
            jstr(&raw, "validator"),
            jstr(&raw, "vote"),
            jf(&raw, "observed_delta"),
            jstr(&raw, "replay_evidence_ref"),
            ju(&raw, "timestamp"),
        );
        check_signature(sim, jstr(&raw, "validator"), &message, &sig)?;
        sim.record_attestation(attestation)?;
        Ok(serde_json::json!({
            "status": "attestation_recorded",
            "block_id": block_id,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `evaluate-block <block-id>`
pub fn cmd_evaluate_block(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: evaluate-block requires a block ID (hex)");
        std::process::exit(1);
    });

    let block_id = match parse_block_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        let outcome = sim.evaluate_block(&block_id)?;
        Ok(serde_json::json!({
            "block_id": block_id,
            "outcome": format!("{:?}", outcome),
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `close-challenge-window <block-id>`
pub fn cmd_close_challenge_window(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: close-challenge-window requires a block ID (hex)");
        std::process::exit(1);
    });

    let block_id = match parse_block_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.close_challenge_window(&block_id)?;
        Ok(serde_json::json!({
            "status": "challenge_window_closed",
            "block_id": block_id,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `settle-block <block-id>`
pub fn cmd_settle_block(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: settle-block requires a block ID (hex)");
        std::process::exit(1);
    });

    let block_id = match parse_block_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.settle_block(&block_id)?;
        Ok(serde_json::json!({
            "status": "block_settled",
            "block_id": block_id,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `finalize-block <block-id>`
pub fn cmd_finalize_block(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: finalize-block requires a block ID (hex)");
        std::process::exit(1);
    });

    let block_id = match parse_block_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.finalize_block(&block_id)?;
        Ok(serde_json::json!({
            "status": "block_finalized",
            "block_id": block_id,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Parameters for `open-challenge`, supplied as JSON.
///
/// Maps to the individual parameters of `SimulatorState::open_challenge`.
/// The CLI caller must construct this explicitly — no convenience defaults.
#[derive(Debug, Deserialize)]
pub struct OpenChallengeParams {
    pub challenge_id: ChallengeId,
    pub challenge_type: ChallengeType,
    pub target: ChallengeTarget,
    pub challenger: ParticipantId,
    pub bond: TokenAmount,
    pub evidence_ref: ArtifactHash,
}

/// `open-challenge <json-file>`
pub fn cmd_open_challenge(state_path: &Path, args: &[String]) {
    let json_file = args.first().unwrap_or_else(|| {
        eprintln!("error: open-challenge requires a JSON file argument");
        std::process::exit(1);
    });

    let (raw, sig) = match load_signed_json(json_file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    let params: OpenChallengeParams = match serde_json::from_value(raw.clone()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: cannot parse {}: {}", json_file, e);
            std::process::exit(1);
        }
    };

    // The signed target is the block ID nested in the target variant
    // (Block/Attestation/Attribution all carry one).
    let target_hex = raw
        .get("target")
        .and_then(|t| t.as_object())
        .and_then(|o| o.values().next())
        .and_then(|inner| inner.get("block_id").or_else(|| inner.get("domain_id")))
        .and_then(|b| b.as_str())
        .unwrap_or("")
        .to_string();

    match load_mutate_save(state_path, |sim| {
        let message = arc_identity::challenge_message(
            jstr(&raw, "challenge_id"),
            jstr(&raw, "challenge_type"),
            &target_hex,
            jstr(&raw, "challenger"),
            ju(&raw, "bond"),
            jstr(&raw, "evidence_ref"),
        );
        check_signature(sim, jstr(&raw, "challenger"), &message, &sig)?;
        let id = sim.open_challenge(
            params.challenge_id,
            params.challenge_type,
            params.target,
            params.challenger,
            params.bond,
            params.evidence_ref,
        )?;
        Ok(serde_json::json!({ "challenge_id": id }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `advance-epoch`
pub fn cmd_advance_epoch(state_path: &Path) {
    match load_mutate_save(state_path, |sim| {
        sim.advance_epoch();
        Ok(serde_json::json!({ "epoch": sim.current_epoch }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `begin-review <challenge-id>`
pub fn cmd_begin_challenge_review(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: begin-review requires a challenge ID (hex)");
        std::process::exit(1);
    });

    let challenge_id = match parse_challenge_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.begin_challenge_review(&challenge_id)?;
        Ok(serde_json::json!({
            "status": "challenge_under_review",
            "challenge_id": challenge_id,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `uphold-challenge <challenge-id>`
pub fn cmd_uphold_challenge(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: uphold-challenge requires a challenge ID (hex)");
        std::process::exit(1);
    });

    let challenge_id = match parse_challenge_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.uphold_challenge(&challenge_id)?;
        let mut result = serde_json::json!({
            "status": "challenge_upheld",
            "challenge_id": challenge_id,
        });
        // Include the slash distribution (challenger payout, burned
        // residual) when the upheld challenge slashed a block.
        if let Some(dist) = sim.slash_distribution(&challenge_id) {
            result["slash_distribution"] = serde_json::to_value(dist)
                .map_err(|e| e.to_string())?;
        }
        Ok(result)
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `expire-challenge <challenge-id>`
pub fn cmd_expire_challenge(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: expire-challenge requires a challenge ID (hex)");
        std::process::exit(1);
    });

    let challenge_id = match parse_challenge_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.expire_challenge(&challenge_id)?;
        Ok(serde_json::json!({
            "status": "challenge_expired",
            "challenge_id": challenge_id,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `reject-challenge <challenge-id>`
pub fn cmd_reject_challenge(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: reject-challenge requires a challenge ID (hex)");
        std::process::exit(1);
    });

    let challenge_id = match parse_challenge_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.reject_challenge(&challenge_id)?;
        Ok(serde_json::json!({
            "status": "challenge_rejected",
            "challenge_id": challenge_id,
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

/// `keygen [out-file]`
///
/// Generate an Ed25519 keypair. The public key is the participant ID.
/// Writes `{"secret": hex32, "public": hex32}` to the file (or stdout).
pub fn cmd_keygen(args: &[String]) {
    let kp = arc_identity::Keypair::generate();
    let hex = |b: &[u8]| b.iter().map(|x| format!("{:02x}", x)).collect::<String>();
    let out = serde_json::json!({
        "secret": hex(&kp.secret_bytes()),
        "public": hex(&kp.public_bytes()),
        "participant_id": hex(&kp.public_bytes()),
    });
    match args.first() {
        Some(path) => {
            if let Err(e) = std::fs::write(path, serde_json::to_string_pretty(&out).unwrap()) {
                eprintln!("error: cannot write {}: {}", path, e);
                std::process::exit(1);
            }
            eprintln!("Keypair written to {}", path);
            println!("{}", serde_json::json!({ "participant_id": out["participant_id"] }));
        }
        None => print_json(&out),
    }
}

/// `top-up-pool <domain-id> <amount>`
///
/// Permissionlessly top up a funded domain's reward pool.
pub fn cmd_top_up_pool(state_path: &Path, args: &[String]) {
    if args.len() < 2 {
        eprintln!("error: top-up-pool requires <domain-id> <amount>");
        std::process::exit(1);
    }
    let domain_id = match parse_hex_bytes(&args[0]) {
        Ok(b) => DomainId::from_bytes(b),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    let amount: u64 = match args[1].parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: invalid amount: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
        sim.top_up_pool(&domain_id, TokenAmount::new(amount))?;
        let pool = sim.domain_pool(&domain_id).unwrap();
        Ok(serde_json::json!({
            "status": "pool_topped_up",
            "domain_id": domain_id,
            "balance": pool.balance,
            "dormant": pool.is_dormant(),
        }))
    }) {
        Ok(result) => print_json(&result),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

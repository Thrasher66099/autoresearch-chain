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

    let genesis: GenesisBlock = match load_json_file(json_file) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
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

    let record: SeedValidationRecord = match load_json_file(&args[1]) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
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

    let block: Block = match load_json_file(json_file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
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

    let attestation: ValidationAttestation = match load_json_file(json_file) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let block_id = attestation.block_id;

    match load_mutate_save(state_path, |sim| {
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

    let params: OpenChallengeParams = match load_json_file(json_file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    match load_mutate_save(state_path, |sim| {
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

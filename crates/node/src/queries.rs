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

//! Read-only query command implementations.
//!
//! Each command loads state, reads requested data, and prints JSON to stdout.
//! No state mutations occur.

use std::path::Path;

use arc_protocol_types::*;
use arc_simulator::state::SimulatorState;

use crate::persistence;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

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

fn parse_domain_id(hex: &str) -> Result<DomainId, String> {
    Ok(DomainId::from_bytes(parse_hex_bytes(hex)?))
}

fn parse_challenge_id(hex: &str) -> Result<ChallengeId, String> {
    Ok(ChallengeId::from_bytes(parse_hex_bytes(hex)?))
}

fn load_state(state_path: &Path) -> SimulatorState {
    match persistence::load_state(state_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_json<T: serde::Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("serialization failed")
    );
}

// -----------------------------------------------------------------------
// Query implementations
// -----------------------------------------------------------------------

/// `list-domains`
pub fn cmd_list_domains(state_path: &Path) {
    let state = load_state(state_path);

    let domains: Vec<serde_json::Value> = state
        .domain_registry
        .domains
        .iter()
        .map(|(id, domain)| {
            serde_json::json!({
                "domain_id": id,
                "name": domain.name,
                "domain_type": format!("{:?}", domain.domain_type),
                "spec_id": domain.spec_id,
            })
        })
        .collect();

    print_json(&serde_json::json!({
        "domain_count": domains.len(),
        "domains": domains,
    }));
}

/// `show-block <block-id>`
pub fn cmd_show_block(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: show-block requires a block ID (hex)");
        std::process::exit(1);
    });

    let block_id = match parse_block_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let state = load_state(state_path);

    let block = match state.blocks.get(&block_id) {
        Some(b) => b,
        None => {
            eprintln!("error: block {} not found", block_id);
            std::process::exit(1);
        }
    };

    let derived_validity = state.derived_validity(&block_id);
    let escrow = state.block_escrow(&block_id);
    let validated_outcome = state.validated_outcome(&block_id);

    let mut result = serde_json::json!({
        "block": block,
        "derived_validity": format!("{:?}", derived_validity),
    });

    if let Some(escrow) = escrow {
        result["escrow"] = serde_json::to_value(escrow).unwrap();
    }
    if let Some(outcome) = validated_outcome {
        result["validated_outcome"] = serde_json::to_value(outcome).unwrap();
    }

    print_json(&result);
}

/// `show-frontier <domain-id>`
pub fn cmd_show_frontier(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: show-frontier requires a domain ID (hex)");
        std::process::exit(1);
    });

    let domain_id = match parse_domain_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let state = load_state(state_path);

    let frontier = state.canonical_frontier(&domain_id);
    let families = state.fork_families(&domain_id);

    let families_json: Vec<serde_json::Value> = families
        .iter()
        .map(|f| serde_json::to_value(f).unwrap())
        .collect();

    print_json(&serde_json::json!({
        "domain_id": domain_id,
        "canonical_frontier": frontier,
        "fork_families": families_json,
    }));
}

/// `show-challenge <challenge-id>`
pub fn cmd_show_challenge(state_path: &Path, args: &[String]) {
    let hex = args.first().unwrap_or_else(|| {
        eprintln!("error: show-challenge requires a challenge ID (hex)");
        std::process::exit(1);
    });

    let challenge_id = match parse_challenge_id(hex) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let state = load_state(state_path);

    let challenge = match state.challenges.get(&challenge_id) {
        Some(c) => c,
        None => {
            eprintln!("error: challenge {} not found", challenge_id);
            std::process::exit(1);
        }
    };

    print_json(challenge);
}

/// `list-blocks [--domain <domain-id>]`
pub fn cmd_list_blocks(state_path: &Path, args: &[String]) {
    let state = load_state(state_path);

    // Parse optional --domain filter.
    let domain_filter = if args.len() >= 2 && args[0] == "--domain" {
        match parse_domain_id(&args[1]) {
            Ok(id) => Some(id),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let blocks: Vec<serde_json::Value> = state
        .blocks
        .iter()
        .filter(|(_, block)| {
            domain_filter
                .map(|did| block.domain_id == did)
                .unwrap_or(true)
        })
        .map(|(id, block)| {
            serde_json::json!({
                "block_id": id,
                "domain_id": block.domain_id,
                "status": format!("{:?}", block.status),
                "parent_id": block.parent_id,
                "claimed_metric_delta": block.claimed_metric_delta,
            })
        })
        .collect();

    print_json(&serde_json::json!({
        "block_count": blocks.len(),
        "blocks": blocks,
    }));
}

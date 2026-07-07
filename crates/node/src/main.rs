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

//! Minimal local runtime for AutoResearch Chain.
//!
//! This binary wraps the protocol core into a single-node local runtime.
//! It is a Phase 1 target — the protocol core must be validated in the
//! simulator first.
//!
//! Current capabilities:
//! - `init`: create a fresh protocol state file
//! - `inspect`: display basic info about a saved state
//! - Transaction submission: submit-genesis, evaluate-conformance,
//!   record-seed-validation, finalize-activation, register-validators,
//!   submit-block, assign-validators, submit-attestation, evaluate-block,
//!   close-challenge-window, settle-block, finalize-block, open-challenge,
//!   begin-review, uphold-challenge, reject-challenge, expire-challenge,
//!   advance-epoch
//! - State queries: list-domains, show-block, show-frontier,
//!   show-challenge, list-blocks
//!
//! Networking is explicitly not the first priority. The node initially
//! runs as a local-only process. Python runners invoke it as a subprocess.

mod commands;
mod ordering;
mod persistence;
mod queries;
mod server;
mod txapply;

use std::path::PathBuf;

use arc_simulator::state::SimulatorState;

const DEFAULT_STATE_FILE: &str = "arc-state.json";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Extract --state flag if present, otherwise use default.
    let (state_path, remaining) = extract_state_flag(&args[1..]);

    if remaining.is_empty() {
        print_usage();
        return;
    }

    let command = remaining[0].as_str();
    let cmd_args = &remaining[1..];

    match command {
        // Existing commands.
        "init" => cmd_init(cmd_args, &state_path),
        "keygen" => commands::cmd_keygen(cmd_args),
        "inspect" => cmd_inspect(cmd_args, &state_path),

        // Networked node (Milestone E2/E3).
        "serve" => cmd_serve(cmd_args, &state_path),
        "follow" => cmd_follow(cmd_args, &state_path),

        // Write commands (mutate state).
        "submit-genesis" => commands::cmd_submit_genesis(&state_path, cmd_args),
        "evaluate-conformance" => commands::cmd_evaluate_conformance(&state_path, cmd_args),
        "record-seed-validation" => commands::cmd_record_seed_validation(&state_path, cmd_args),
        "finalize-activation" => commands::cmd_finalize_activation(&state_path, cmd_args),
        "register-validators" => commands::cmd_register_validators(&state_path, cmd_args),
        "submit-block" => commands::cmd_submit_block(&state_path, cmd_args),
        "assign-validators" => commands::cmd_assign_validators(&state_path, cmd_args),
        "submit-attestation" => commands::cmd_submit_attestation(&state_path, cmd_args),
        "evaluate-block" => commands::cmd_evaluate_block(&state_path, cmd_args),
        "close-challenge-window" => commands::cmd_close_challenge_window(&state_path, cmd_args),
        "settle-block" => commands::cmd_settle_block(&state_path, cmd_args),
        "finalize-block" => commands::cmd_finalize_block(&state_path, cmd_args),
        "open-challenge" => commands::cmd_open_challenge(&state_path, cmd_args),
        "begin-review" => commands::cmd_begin_challenge_review(&state_path, cmd_args),
        "uphold-challenge" => commands::cmd_uphold_challenge(&state_path, cmd_args),
        "reject-challenge" => commands::cmd_reject_challenge(&state_path, cmd_args),
        "expire-challenge" => commands::cmd_expire_challenge(&state_path, cmd_args),
        "advance-epoch" => commands::cmd_advance_epoch(&state_path),
        "top-up-pool" => commands::cmd_top_up_pool(&state_path, cmd_args),

        // Read commands (query state).
        "list-domains" => queries::cmd_list_domains(&state_path),
        "show-block" => queries::cmd_show_block(&state_path, cmd_args),
        "show-frontier" => queries::cmd_show_frontier(&state_path, cmd_args),
        "show-challenge" => queries::cmd_show_challenge(&state_path, cmd_args),
        "list-blocks" => queries::cmd_list_blocks(&state_path, cmd_args),
        "show-pool" => queries::cmd_show_pool(&state_path, cmd_args),

        "help" | "--help" | "-h" => print_usage(),
        _ => {
            eprintln!("error: unknown command: {}", command);
            eprintln!();
            print_usage();
            std::process::exit(1);
        }
    }
}

/// Extract `--state <path>` from arguments, returning the state path and
/// remaining arguments with the flag removed.
fn extract_state_flag(args: &[String]) -> (PathBuf, Vec<String>) {
    let mut state_path = PathBuf::from(DEFAULT_STATE_FILE);
    let mut remaining = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--state" {
            if i + 1 < args.len() {
                state_path = PathBuf::from(&args[i + 1]);
                i += 2;
                continue;
            } else {
                eprintln!("error: --state requires a path argument");
                std::process::exit(1);
            }
        }
        remaining.push(args[i].clone());
        i += 1;
    }
    (state_path, remaining)
}

fn print_usage() {
    eprintln!("arc-node: minimal local runtime for AutoResearch Chain");
    eprintln!();
    eprintln!("Usage: arc-node [--state <path>] <command> [args...]");
    eprintln!();
    eprintln!("State management:");
    eprintln!("  init [PATH] [--require-signatures]  Create a fresh protocol state file");
    eprintln!("  keygen [out-file]                Generate an Ed25519 keypair (public key = participant ID)");
    eprintln!("  inspect [PATH]                   Display basic info about a saved state");
    eprintln!();
    eprintln!("Networked node (single-sequencer PoA, temporary scaffolding):");
    eprintln!("  serve --authority-key <file> [--listen HOST:PORT] [--store DIR]");
    eprintln!("                                   Run the sequencer HTTP server");
    eprintln!("  follow --sequencer <url> --authority <pubkey-hex> [--once]");
    eprintln!("                                   Sync and verify the ordering log");
    eprintln!();
    eprintln!("Genesis / domain activation:");
    eprintln!("  submit-genesis <json-file>                Submit a genesis proposal");
    eprintln!("  evaluate-conformance <genesis-id>          Run RTS conformance check");
    eprintln!("  record-seed-validation <genesis-id> <json> Record a seed validation");
    eprintln!("  finalize-activation <genesis-id>           Finalize track activation");
    eprintln!("  register-validators <json-file>            Register a validator pool");
    eprintln!();
    eprintln!("Block lifecycle:");
    eprintln!("  submit-block <json-file>         Submit a block");
    eprintln!("  assign-validators <block-id>     Assign validators and begin validation");
    eprintln!("  submit-attestation <json-file>   Record a validation attestation");
    eprintln!("  evaluate-block <block-id>        Aggregate attestations and evaluate");
    eprintln!("  close-challenge-window <block-id> Close challenge window");
    eprintln!("  settle-block <block-id>          Settle block and release escrow");
    eprintln!("  finalize-block <block-id>        Finalize a settled block");
    eprintln!();
    eprintln!("Challenges:");
    eprintln!("  open-challenge <json-file>       Open a challenge");
    eprintln!("  begin-review <challenge-id>      Begin challenge review");
    eprintln!("  uphold-challenge <challenge-id>   Uphold a challenge (invalidate target, pay challenger)");
    eprintln!("  reject-challenge <challenge-id>   Reject a challenge (challenger loses bond)");
    eprintln!("  expire-challenge <challenge-id>   Expire an unresolved challenge (bond returned)");
    eprintln!();
    eprintln!("Epoch:");
    eprintln!("  advance-epoch                    Advance to the next epoch");
    eprintln!("  top-up-pool <domain-id> <amount> Top up a funded domain reward pool");
    eprintln!();
    eprintln!("Queries:");
    eprintln!("  list-domains                     List all registered domains");
    eprintln!("  show-block <block-id>            Show block details");
    eprintln!("  show-frontier <domain-id>        Show canonical frontier and fork families");
    eprintln!("  show-challenge <challenge-id>    Show challenge details");
    eprintln!("  list-blocks [--domain <id>]      List blocks (optionally filtered)");
    eprintln!();
    eprintln!("  help                             Show this message");
    eprintln!();
    eprintln!("Default state file: {}", DEFAULT_STATE_FILE);
    eprintln!("All IDs are 64-character lowercase hex strings.");
    eprintln!("All output is JSON to stdout; errors go to stderr.");
}

fn cmd_init(args: &[String], default_state_path: &PathBuf) {
    // init accepts an optional positional path and/or --require-signatures.
    let require_signatures = args.iter().any(|a| a == "--require-signatures");
    let path = if let Some(pos) = args.iter().find(|a| !a.starts_with("--")) {
        PathBuf::from(pos)
    } else {
        default_state_path.clone()
    };

    if path.exists() {
        eprintln!("error: state file already exists: {}", path.display());
        eprintln!("Remove it first if you want to reinitialize.");
        std::process::exit(1);
    }

    let mut state = SimulatorState::new();
    state.require_signatures = require_signatures;
    if require_signatures {
        eprintln!("Signature enforcement: ON (actor-bearing submissions must be signed)");
    }
    match persistence::save_state(&state, &path) {
        Ok(()) => {
            eprintln!("Created fresh state: {}", path.display());
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_inspect(args: &[String], default_state_path: &PathBuf) {
    let path = if let Some(pos) = args.first() {
        PathBuf::from(pos)
    } else {
        default_state_path.clone()
    };

    let state = match persistence::load_state(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    eprintln!("State file: {}", path.display());
    eprintln!("  Epoch:              {}", state.current_epoch);
    eprintln!("  Active domains:     {}", state.domain_registry.domains.len());
    eprintln!("  Blocks:             {}", state.blocks.len());
    eprintln!("  Challenges:         {}", state.challenges.len());
    eprintln!("  Escrow records:     {}", state.escrow_records.len());
    eprintln!("  Pending activations:{}", state.pending_activations.len());
}

/// Read a flag's value from args (`--name value`).
fn flag_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn cmd_serve(args: &[String], state_path: &PathBuf) {
    let listen = flag_value(args, "--listen").unwrap_or_else(|| "127.0.0.1:8730".to_string());
    let key_file = flag_value(args, "--authority-key").unwrap_or_else(|| {
        eprintln!("error: serve requires --authority-key <key-file> (from keygen)");
        std::process::exit(1);
    });
    let key_json: serde_json::Value = match std::fs::read_to_string(&key_file)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: cannot read key file {}: {}", key_file, e);
            std::process::exit(1);
        }
    };
    let secret_hex = key_json["secret"].as_str().unwrap_or("");
    let mut secret = [0u8; 32];
    if secret_hex.len() != 64 {
        eprintln!("error: key file must contain a 64-hex-char `secret`");
        std::process::exit(1);
    }
    for i in 0..32 {
        secret[i] = u8::from_str_radix(&secret_hex[i * 2..i * 2 + 2], 16).unwrap_or(0);
    }

    let config = server::ServeConfig {
        state_path: state_path.clone(),
        listen,
        authority: arc_identity::Keypair::from_secret_bytes(&secret),
        store_dir: flag_value(args, "--store").map(PathBuf::from),
    };
    let max_requests = flag_value(args, "--max-requests").and_then(|v| v.parse().ok());
    if let Err(e) = server::serve(config, max_requests) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_follow(args: &[String], state_path: &PathBuf) {
    let sequencer = flag_value(args, "--sequencer").unwrap_or_else(|| {
        eprintln!("error: follow requires --sequencer <url>");
        std::process::exit(1);
    });
    let authority = flag_value(args, "--authority").unwrap_or_else(|| {
        eprintln!("error: follow requires --authority <pubkey-hex>");
        std::process::exit(1);
    });
    let once = args.iter().any(|a| a == "--once");
    loop {
        match server::follow_once(state_path, &sequencer, &authority) {
            Ok(result) => {
                println!("{}", serde_json::to_string(&result).unwrap());
                if once {
                    break;
                }
            }
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

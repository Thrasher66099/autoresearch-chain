// SPDX-License-Identifier: AGPL-3.0-or-later

//! End-to-end CLI integration tests for arc-node.
//!
//! These tests invoke the arc-node binary as a subprocess to verify
//! that all transaction submission and query commands work correctly
//! through the full CLI interface. They mirror the scenario test
//! patterns from `crates/simulator/tests/scenarios.rs`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Path to the arc-node binary built by cargo.
fn arc_node_bin() -> PathBuf {
    // cargo test builds into target/debug by default.
    let path = PathBuf::from(env!("CARGO_BIN_EXE_arc-node"));
    assert!(path.exists(), "arc-node binary not found at {:?}", path);
    path
}

/// Create a unique temp directory for a test.
fn test_dir(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("arc_node_integration_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run arc-node with the given arguments and return (stdout, stderr, success).
fn run_node(args: &[&str]) -> (String, String, bool) {
    let output = Command::new(arc_node_bin())
        .args(args)
        .output()
        .expect("failed to execute arc-node");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

/// Run arc-node and assert success; return parsed stdout JSON.
fn run_ok(args: &[&str]) -> serde_json::Value {
    let (stdout, stderr, success) = run_node(args);
    assert!(
        success,
        "arc-node {:?} failed.\nstderr: {}\nstdout: {}",
        args, stderr, stdout
    );
    if stdout.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "failed to parse stdout as JSON: {}\nstdout: {}",
                e, stdout
            )
        })
    }
}

/// Run arc-node and assert failure.
fn run_err(args: &[&str]) -> String {
    let (stdout, stderr, success) = run_node(args);
    assert!(
        !success,
        "arc-node {:?} should have failed.\nstdout: {}\nstderr: {}",
        args, stdout, stderr
    );
    stderr
}

/// Helper: hex string for `[n; 32]`.
fn hex_id(n: u8) -> String {
    format!("{:02x}", n).repeat(32)
}

/// Write JSON content to a file in the given directory.
fn write_json(dir: &PathBuf, name: &str, value: &serde_json::Value) -> String {
    let path = dir.join(name);
    fs::write(&path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    path.to_str().unwrap().to_string()
}

/// Build a valid genesis block JSON matching `valid_genesis_block()` from fixtures.
fn genesis_json() -> serde_json::Value {
    serde_json::json!({
        "id": hex_id(1),
        "rts_version": "Rts1",
        "domain_id": hex_id(1),
        "proposer": hex_id(1),
        "research_target_declaration": "Improve CIFAR-10 training recipe accuracy within fixed compute budget",
        "domain_intent": "EndToEndRecipeImprovement",
        "seed_recipe_ref": hex_id(10),
        "seed_codebase_state_ref": hex_id(11),
        "frozen_surface": ["eval/", "datasets/"],
        "search_surface": ["train.py", "config/", "models/"],
        "canonical_dataset_ref": hex_id(20),
        "dataset_hash": hex_id(21),
        "dataset_splits": {
            "training": hex_id(22),
            "validation": hex_id(23),
            "test": hex_id(24)
        },
        "evaluation_harness_ref": hex_id(30),
        "metric_id": "test_accuracy",
        "metric_direction": "HigherBetter",
        "hardware_class": "RTX 4090",
        "time_budget_secs": 3600,
        "seed_environment_manifest_ref": hex_id(40),
        "seed_score": 0.93,
        "artifact_schema_ref": hex_id(50),
        "seed_bond": 1000,
        "license_declaration": "MIT",
        "timestamp": 1700000000
    })
}

/// Build a seed validation record JSON.
fn seed_validation_json(validator_n: u8) -> serde_json::Value {
    serde_json::json!({
        "validator": hex_id(validator_n),
        "vote": "Pass",
        "observed_score": 0.93,
        "timestamp": 1700000000 + validator_n as u64
    })
}

/// Build a validator pool JSON.
fn validator_pool_json() -> serde_json::Value {
    let validators: Vec<String> = (1..=10).map(|i| hex_id(i)).collect();
    serde_json::json!({
        "domain_id": hex_id(1),
        "validators": validators
    })
}

/// Build a block JSON.
fn block_json(id: u8, parent_id: &str, domain_id: &str, delta: f64) -> serde_json::Value {
    serde_json::json!({
        "id": hex_id(id),
        "domain_id": domain_id,
        "parent_id": parent_id,
        "proposer": hex_id(1),
        "child_state_ref": hex_id(60 + id),
        "diff_ref": hex_id(160 + id),
        "claimed_metric_delta": delta,
        "evidence_bundle_hash": hex_id(200 + id),
        "fee": 10,
        "bond": 500,
        "epoch_id": 1,
        "status": "Submitted",
        "timestamp": 1700001000 + id as u64 * 1000
    })
}

/// Build an attestation JSON.
fn attestation_json(
    block_id: &str,
    validator_id: &str,
    observed_delta: f64,
) -> serde_json::Value {
    serde_json::json!({
        "block_id": block_id,
        "validator": validator_id,
        "vote": "Pass",
        "observed_delta": observed_delta,
        "replay_evidence_ref": hex_id(70),
        "timestamp": 1700002000
    })
}

/// Build an open-challenge params JSON.
fn challenge_params_json(
    challenge_n: u8,
    block_id: &str,
    challenger_n: u8,
) -> serde_json::Value {
    serde_json::json!({
        "challenge_id": hex_id(challenge_n),
        "challenge_type": "BlockReplay",
        "target": { "Block": { "block_id": block_id } },
        "challenger": hex_id(challenger_n),
        "bond": 200,
        "evidence_ref": hex_id(80)
    })
}

/// Set up an active domain with a validated block via CLI commands.
/// Returns (block_id_hex, assigned_validators).
fn setup_domain_with_block(
    dir: &PathBuf,
    state_str: &str,
) -> (String, Vec<serde_json::Value>) {
    let genesis_id = hex_id(1);
    let domain_id = hex_id(1);

    let genesis_file = write_json(dir, "genesis.json", &genesis_json());
    run_ok(&["--state", state_str, "submit-genesis", &genesis_file]);
    run_ok(&["--state", state_str, "evaluate-conformance", &genesis_id]);
    for i in 1..=3u8 {
        let sv_file = write_json(dir, &format!("sv_{}.json", i), &seed_validation_json(i));
        run_ok(&["--state", state_str, "record-seed-validation", &genesis_id, &sv_file]);
    }
    run_ok(&["--state", state_str, "finalize-activation", &genesis_id]);
    let pool_file = write_json(dir, "pool.json", &validator_pool_json());
    run_ok(&["--state", state_str, "register-validators", &pool_file]);

    let block_id = hex_id(10);
    let block_file = write_json(
        dir,
        "block.json",
        &block_json(10, &genesis_id, &domain_id, 0.015),
    );
    run_ok(&["--state", state_str, "submit-block", &block_file]);
    let result = run_ok(&["--state", state_str, "assign-validators", &block_id]);
    let assigned = result["assigned_validators"].as_array().unwrap().clone();
    for (idx, v) in assigned.iter().enumerate() {
        let att_file = write_json(
            dir,
            &format!("att_{}.json", idx),
            &attestation_json(&block_id, v.as_str().unwrap(), 0.015),
        );
        run_ok(&["--state", state_str, "submit-attestation", &att_file]);
    }
    run_ok(&["--state", state_str, "evaluate-block", &block_id]);

    (block_id, assigned)
}

// =======================================================================
// Tests
// =======================================================================

#[test]
fn test_init_and_inspect() {
    let dir = test_dir("init_inspect");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    // Init creates state file.
    let (_, stderr, success) = run_node(&["init", state_str]);
    assert!(success, "init failed: {}", stderr);
    assert!(state.exists());

    // Inspect reads it.
    let (_, stderr, success) = run_node(&["inspect", state_str]);
    assert!(success, "inspect failed: {}", stderr);
    assert!(stderr.contains("Epoch:"));

    // Init fails if file already exists.
    let stderr = run_err(&["init", state_str]);
    assert!(stderr.contains("already exists"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_full_block_lifecycle() {
    let dir = test_dir("full_lifecycle");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    // Init.
    run_node(&["init", state_str]);

    let genesis_id = hex_id(1);
    let domain_id = hex_id(1);

    // Submit genesis.
    let genesis_file = write_json(&dir, "genesis.json", &genesis_json());
    let result = run_ok(&["--state", state_str, "submit-genesis", &genesis_file]);
    assert_eq!(result["genesis_id"], genesis_id);

    // Evaluate conformance.
    let result = run_ok(&[
        "--state", state_str,
        "evaluate-conformance", &genesis_id,
    ]);
    assert_eq!(result["status"], "conformance_passed");

    // Record 3 seed validations.
    for i in 1..=3u8 {
        let sv_file = write_json(
            &dir,
            &format!("sv_{}.json", i),
            &seed_validation_json(i),
        );
        let result = run_ok(&[
            "--state", state_str,
            "record-seed-validation", &genesis_id, &sv_file,
        ]);
        assert_eq!(result["status"], "seed_validation_recorded");
    }

    // Finalize activation.
    let result = run_ok(&[
        "--state", state_str,
        "finalize-activation", &genesis_id,
    ]);
    assert_eq!(result["status"], "domain_activated");
    assert_eq!(result["domain_id"], domain_id);

    // Register validators.
    let pool_file = write_json(&dir, "pool.json", &validator_pool_json());
    let result = run_ok(&[
        "--state", state_str,
        "register-validators", &pool_file,
    ]);
    assert_eq!(result["status"], "validators_registered");
    assert_eq!(result["validator_count"], 10);

    // Query: list-domains should show 1 domain.
    let result = run_ok(&["--state", state_str, "list-domains"]);
    assert_eq!(result["domain_count"], 1);

    // Submit a block.
    // Parent is the genesis block ID (same bytes).
    let block_id = hex_id(10);
    let block_file = write_json(
        &dir,
        "block.json",
        &block_json(10, &genesis_id, &domain_id, 0.015),
    );
    let result = run_ok(&["--state", state_str, "submit-block", &block_file]);
    assert_eq!(result["block_id"], block_id);

    // Assign validators.
    let result = run_ok(&[
        "--state", state_str,
        "assign-validators", &block_id,
    ]);
    assert_eq!(result["block_id"], block_id);
    let assigned = result["assigned_validators"].as_array().unwrap();
    assert_eq!(assigned.len(), 3); // default validators_per_block

    // Submit attestations from each assigned validator.
    for (idx, v) in assigned.iter().enumerate() {
        let validator_hex = v.as_str().unwrap();
        let att_file = write_json(
            &dir,
            &format!("att_{}.json", idx),
            &attestation_json(&block_id, validator_hex, 0.015),
        );
        let result = run_ok(&[
            "--state", state_str,
            "submit-attestation", &att_file,
        ]);
        assert_eq!(result["status"], "attestation_recorded");
    }

    // Evaluate block.
    let result = run_ok(&[
        "--state", state_str,
        "evaluate-block", &block_id,
    ]);
    assert_eq!(result["outcome"], "Accepted");

    // Query: show-block should return block details.
    let result = run_ok(&["--state", state_str, "show-block", &block_id]);
    assert!(result["block"].is_object());
    assert_eq!(result["derived_validity"], "DirectValid");
    assert!(result["escrow"].is_object());
    assert!(result["validated_outcome"].is_object());

    // Query: show-frontier should show this block as frontier.
    let result = run_ok(&["--state", state_str, "show-frontier", &domain_id]);
    assert_eq!(result["canonical_frontier"], block_id);

    // Query: list-blocks should show 1 block.
    let result = run_ok(&["--state", state_str, "list-blocks"]);
    assert_eq!(result["block_count"], 1);

    // Query: list-blocks with domain filter.
    let result = run_ok(&[
        "--state", state_str,
        "list-blocks", "--domain", &domain_id,
    ]);
    assert_eq!(result["block_count"], 1);

    // Close challenge window.
    let result = run_ok(&[
        "--state", state_str,
        "close-challenge-window", &block_id,
    ]);
    assert_eq!(result["status"], "challenge_window_closed");

    // Advance epoch 5 times (to reach release_epoch).
    for _ in 0..5 {
        run_ok(&["--state", state_str, "advance-epoch"]);
    }

    // Settle block.
    let result = run_ok(&[
        "--state", state_str,
        "settle-block", &block_id,
    ]);
    assert_eq!(result["status"], "block_settled");

    // Finalize block.
    let result = run_ok(&[
        "--state", state_str,
        "finalize-block", &block_id,
    ]);
    assert_eq!(result["status"], "block_finalized");

    // Inspect should show updated state.
    let (_, stderr, success) = run_node(&["inspect", state_str]);
    assert!(success, "inspect failed: {}", stderr);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_early_settlement_rejected() {
    let dir = test_dir("early_settle");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    // Set up active domain with an accepted block.
    run_node(&["init", state_str]);
    let genesis_id = hex_id(1);
    let domain_id = hex_id(1);

    let genesis_file = write_json(&dir, "genesis.json", &genesis_json());
    run_ok(&["--state", state_str, "submit-genesis", &genesis_file]);
    run_ok(&["--state", state_str, "evaluate-conformance", &genesis_id]);
    for i in 1..=3u8 {
        let sv_file = write_json(&dir, &format!("sv_{}.json", i), &seed_validation_json(i));
        run_ok(&["--state", state_str, "record-seed-validation", &genesis_id, &sv_file]);
    }
    run_ok(&["--state", state_str, "finalize-activation", &genesis_id]);
    let pool_file = write_json(&dir, "pool.json", &validator_pool_json());
    run_ok(&["--state", state_str, "register-validators", &pool_file]);

    // Submit and validate a block.
    let block_id = hex_id(10);
    let block_file = write_json(
        &dir,
        "block.json",
        &block_json(10, &genesis_id, &domain_id, 0.015),
    );
    run_ok(&["--state", state_str, "submit-block", &block_file]);
    let result = run_ok(&["--state", state_str, "assign-validators", &block_id]);
    let assigned = result["assigned_validators"].as_array().unwrap();
    for (idx, v) in assigned.iter().enumerate() {
        let att_file = write_json(
            &dir,
            &format!("att_{}.json", idx),
            &attestation_json(&block_id, v.as_str().unwrap(), 0.015),
        );
        run_ok(&["--state", state_str, "submit-attestation", &att_file]);
    }
    run_ok(&["--state", state_str, "evaluate-block", &block_id]);
    run_ok(&["--state", state_str, "close-challenge-window", &block_id]);

    // Do NOT advance epoch — settlement should fail.
    let stderr = run_err(&["--state", state_str, "settle-block", &block_id]);
    assert!(
        stderr.contains("escrow") || stderr.contains("epoch"),
        "expected escrow/epoch error, got: {}",
        stderr
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_unknown_command_fails() {
    let (_, stderr, success) = run_node(&["nonexistent-command"]);
    assert!(!success);
    assert!(stderr.contains("unknown command"));
}

#[test]
fn test_state_flag_works() {
    let dir = test_dir("state_flag");
    let state = dir.join("custom.json");
    let state_str = state.to_str().unwrap();

    // Init using --state flag.
    let (_, stderr, success) = run_node(&["--state", state_str, "init"]);
    assert!(success, "init with --state failed: {}", stderr);
    assert!(state.exists());

    // Inspect using --state flag.
    let (_, stderr, success) = run_node(&["--state", state_str, "inspect"]);
    assert!(success, "inspect with --state failed: {}", stderr);
    assert!(stderr.contains("Epoch:"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_open_challenge_and_show_challenge() {
    let dir = test_dir("challenge");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);
    let (block_id, _) = setup_domain_with_block(&dir, state_str);

    // Block is UnderChallenge after acceptance — open a challenge.
    let challenge_id = hex_id(1);
    let challenge_file = write_json(
        &dir,
        "challenge.json",
        &challenge_params_json(1, &block_id, 5),
    );
    let result = run_ok(&[
        "--state", state_str,
        "open-challenge", &challenge_file,
    ]);
    assert_eq!(result["challenge_id"], challenge_id);

    // show-challenge should return the challenge record.
    let result = run_ok(&[
        "--state", state_str,
        "show-challenge", &challenge_id,
    ]);
    assert_eq!(result["id"], challenge_id);
    assert_eq!(result["challenge_type"], "BlockReplay");
    assert_eq!(result["status"], "Open");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_advance_epoch_returns_correct_epoch() {
    let dir = test_dir("advance_epoch");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);

    // Epoch starts at 0. Each advance should increment by 1.
    for expected in 1..=3u64 {
        let result = run_ok(&["--state", state_str, "advance-epoch"]);
        assert_eq!(
            result["epoch"], expected,
            "epoch should be {} after {} advances",
            expected, expected
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_invalid_hex_id_rejected() {
    let dir = test_dir("bad_hex");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);

    // Too short.
    let stderr = run_err(&["--state", state_str, "evaluate-conformance", "abcd"]);
    assert!(stderr.contains("64 hex characters"), "stderr: {}", stderr);

    // Non-hex characters.
    let bad_hex = "zz".to_string() + &"00".repeat(31);
    let stderr = run_err(&["--state", state_str, "show-block", &bad_hex]);
    assert!(stderr.contains("invalid hex"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_malformed_json_file_rejected() {
    let dir = test_dir("bad_json");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);

    // Write invalid JSON.
    let bad_file = dir.join("bad.json");
    fs::write(&bad_file, "{ not valid json }").unwrap();
    let bad_str = bad_file.to_str().unwrap();

    let stderr = run_err(&["--state", state_str, "submit-genesis", bad_str]);
    assert!(stderr.contains("cannot parse"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_protocol_rejection_propagates() {
    let dir = test_dir("proto_reject");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);

    // Submit an invalid genesis (empty metric_id).
    let mut bad_genesis = genesis_json();
    bad_genesis["metric_id"] = serde_json::json!("");
    let genesis_file = write_json(&dir, "bad_genesis.json", &bad_genesis);

    let stderr = run_err(&["--state", state_str, "submit-genesis", &genesis_file]);
    assert!(!stderr.is_empty(), "protocol error should appear on stderr");

    // Submit a block against a non-existent domain.
    let block_file = write_json(
        &dir,
        "block.json",
        &block_json(10, &hex_id(99), &hex_id(99), 0.01),
    );
    let stderr = run_err(&["--state", state_str, "submit-block", &block_file]);
    assert!(
        stderr.contains("not active"),
        "expected domain-not-active error, got: {}",
        stderr
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_command_against_missing_state_file() {
    let dir = test_dir("missing_state");
    let bogus = dir.join("does_not_exist.json");
    let bogus_str = bogus.to_str().unwrap();

    let stderr = run_err(&["--state", bogus_str, "advance-epoch"]);
    assert!(
        stderr.contains("I/O error") || stderr.contains("No such file"),
        "expected load error, got: {}",
        stderr
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_show_block_nonexistent() {
    let dir = test_dir("show_missing");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);

    let stderr = run_err(&[
        "--state", state_str,
        "show-block", &hex_id(99),
    ]);
    assert!(stderr.contains("not found"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_show_challenge_nonexistent() {
    let dir = test_dir("show_missing_challenge");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);

    let stderr = run_err(&[
        "--state", state_str,
        "show-challenge", &hex_id(99),
    ]);
    assert!(stderr.contains("not found"), "stderr: {}", stderr);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_show_frontier_empty_domain() {
    let dir = test_dir("empty_frontier");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);

    // Query frontier for a domain that doesn't exist — should return
    // null frontier and empty fork families (not an error).
    let result = run_ok(&[
        "--state", state_str,
        "show-frontier", &hex_id(99),
    ]);
    assert!(result["canonical_frontier"].is_null());
    assert_eq!(result["fork_families"].as_array().unwrap().len(), 0);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_list_blocks_domain_filter_no_match() {
    let dir = test_dir("filter_nomatch");
    let state = dir.join("state.json");
    let state_str = state.to_str().unwrap();

    run_node(&["init", state_str]);
    setup_domain_with_block(&dir, state_str);

    // Filter by a domain that has no blocks.
    let result = run_ok(&[
        "--state", state_str,
        "list-blocks", "--domain", &hex_id(99),
    ]);
    assert_eq!(result["block_count"], 0);

    let _ = fs::remove_dir_all(&dir);
}

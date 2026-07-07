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

//! Sequencer HTTP server (Milestone E3).
//!
//! Endpoints:
//! - `POST /tx/<kind>`      — submit a transaction; on success it is
//!   applied to state and appended to the ordering log atomically.
//! - `GET  /log?from=N`     — ordered entries from sequence N (JSON array).
//! - `GET  /state`          — full state snapshot (JSON).
//! - `GET  /status`         — `{seq, tip_hash, state_hash, authority}`.
//! - `GET  /artifact/<hash>` — content-addressed artifact bytes.
//! - `POST /artifact`       — store artifact bytes; returns its hash.
//!
//! Transport is plain HTTP: the payloads carry their own signatures, the
//! log carries the authority's, so the transport needs no trust.

use std::path::{Path, PathBuf};

use arc_simulator::state::SimulatorState;

use crate::ordering::OrderingLog;
use crate::{persistence, txapply};

pub struct ServeConfig {
    pub state_path: PathBuf,
    pub listen: String,
    pub authority: arc_identity::Keypair,
    pub store_dir: Option<PathBuf>,
}

fn respond_json(req: tiny_http::Request, status: u16, body: &serde_json::Value) {
    let data = serde_json::to_string(body).unwrap();
    let response = tiny_http::Response::from_string(data)
        .with_status_code(status)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                .unwrap(),
        );
    let _ = req.respond(response);
}

fn err_json(req: tiny_http::Request, status: u16, msg: &str) {
    respond_json(req, status, &serde_json::json!({ "error": msg }));
}

/// Run the sequencer. Blocks forever serving requests; `max_requests`
/// (if set) stops after that many handled requests — used by tests.
pub fn serve(config: ServeConfig, max_requests: Option<usize>) -> Result<(), String> {
    let mut sim = persistence::load_state(&config.state_path)
        .map_err(|e| format!("cannot load state: {}", e))?;
    let log_path = config.state_path.with_extension("log.jsonl");
    let mut log = OrderingLog::open(&log_path)?;
    let authority_hex: String = config
        .authority
        .public_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    let server = tiny_http::Server::http(&config.listen)
        .map_err(|e| format!("cannot bind {}: {}", config.listen, e))?;
    eprintln!(
        "Sequencer listening on {} (authority {}, log at seq {})",
        config.listen,
        authority_hex,
        log.next_seq()
    );

    let mut handled = 0usize;
    for mut req in server.incoming_requests() {
        let url = req.url().to_string();
        let method = req.method().clone();
        let path_parts: Vec<&str> = url
            .split('?')
            .next()
            .unwrap_or("")
            .trim_matches('/')
            .split('/')
            .collect();

        match (method, path_parts.as_slice()) {
            (tiny_http::Method::Post, ["tx", kind]) => {
                let kind = kind.to_string();
                let mut body = String::new();
                if req.as_reader().read_to_string(&mut body).is_err() {
                    err_json(req, 400, "unreadable body");
                    continue;
                }
                let payload: serde_json::Value = match serde_json::from_str(&body) {
                    Ok(v) => v,
                    Err(e) => {
                        err_json(req, 400, &format!("invalid JSON: {}", e));
                        continue;
                    }
                };
                // Apply first: only valid transactions enter the log.
                match txapply::apply_tx(&mut sim, &kind, &payload) {
                    Ok(result) => {
                        let entry = log.append(&kind, payload, &config.authority)?;
                        let seq = entry.seq;
                        if let Err(e) = persistence::save_state(&sim, &config.state_path) {
                            err_json(req, 500, &format!("state save failed: {}", e));
                            continue;
                        }
                        respond_json(
                            req,
                            200,
                            &serde_json::json!({ "seq": seq, "result": result }),
                        );
                    }
                    Err(e) => err_json(req, 422, &e),
                }
            }
            (tiny_http::Method::Get, ["log"]) => {
                let from: usize = url
                    .split("from=")
                    .nth(1)
                    .and_then(|s| s.split('&').next())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let entries: Vec<&crate::ordering::LogEntry> =
                    log.entries.iter().skip(from).collect();
                respond_json(req, 200, &serde_json::to_value(entries).unwrap());
            }
            (tiny_http::Method::Get, ["state"]) => {
                respond_json(req, 200, &serde_json::to_value(&sim).unwrap());
            }
            (tiny_http::Method::Get, ["status"]) => {
                respond_json(
                    req,
                    200,
                    &serde_json::json!({
                        "seq": log.next_seq(),
                        "tip_hash": log.tip_hash(),
                        "state_hash": txapply::state_hash(&sim),
                        "authority": authority_hex,
                        "require_signatures": sim.require_signatures,
                    }),
                );
            }
            (tiny_http::Method::Get, ["artifact", hash]) => {
                let Some(store) = &config.store_dir else {
                    err_json(req, 404, "no artifact store configured");
                    continue;
                };
                match read_artifact(store, hash) {
                    Ok(bytes) => {
                        let _ = req.respond(tiny_http::Response::from_data(bytes));
                    }
                    Err(e) => err_json(req, 404, &e),
                }
            }
            (tiny_http::Method::Post, ["artifact"]) => {
                let Some(store) = &config.store_dir else {
                    err_json(req, 404, "no artifact store configured");
                    continue;
                };
                let mut bytes = Vec::new();
                if req.as_reader().read_to_end(&mut bytes).is_err() {
                    err_json(req, 400, "unreadable body");
                    continue;
                }
                let hash = blake3::hash(&bytes).to_hex().to_string();
                if let Err(e) = std::fs::write(store.join(&hash), &bytes) {
                    err_json(req, 500, &format!("store write failed: {}", e));
                    continue;
                }
                respond_json(req, 200, &serde_json::json!({ "hash": hash }));
            }
            _ => err_json(req, 404, "unknown endpoint"),
        }

        handled += 1;
        if let Some(max) = max_requests {
            if handled >= max {
                break;
            }
        }
    }
    Ok(())
}

/// Read a stored artifact, verifying content-addressing on the way out.
fn read_artifact(store: &Path, hash: &str) -> Result<Vec<u8>, String> {
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid artifact hash".to_string());
    }
    let bytes = std::fs::read(store.join(hash))
        .map_err(|_| format!("artifact not found: {}", hash))?;
    if blake3::hash(&bytes).to_hex().to_string() != hash {
        return Err(format!("store corruption for {}", hash));
    }
    Ok(bytes)
}

/// Follower: fetch, verify, and apply the sequencer's log (Milestone E3).
///
/// Verifies the hash chain and authority signature on every entry, then
/// replays each transaction through the same `apply_tx` path the
/// sequencer used (re-verifying actor signatures). On catch-up, compares
/// state hashes and errors on divergence.
pub fn follow_once(
    state_path: &Path,
    sequencer: &str,
    authority_hex: &str,
) -> Result<serde_json::Value, String> {
    let mut sim: SimulatorState = persistence::load_state(state_path)
        .map_err(|e| format!("cannot load state: {}", e))?;
    let log_path = state_path.with_extension("log.jsonl");
    let mut log = OrderingLog::open(&log_path)?;

    let mut authority_pk = [0u8; 32];
    if authority_hex.len() != 64 {
        return Err("authority must be 64 hex chars".to_string());
    }
    for (i, byte) in authority_pk.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&authority_hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| format!("authority hex: {}", e))?;
    }

    let from = log.next_seq();
    let body = ureq::get(&format!("{}/log?from={}", sequencer, from))
        .call()
        .map_err(|e| format!("cannot fetch log: {}", e))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let entries: Vec<crate::ordering::LogEntry> =
        serde_json::from_str(&body).map_err(|e| format!("bad log response: {}", e))?;

    let mut applied = 0u64;
    for entry in entries {
        crate::ordering::verify_entry(&entry, log.next_seq(), &log.tip_hash(), &authority_pk)?;
        txapply::apply_tx(&mut sim, &entry.kind, &entry.payload)
            .map_err(|e| format!("replay failed at seq {}: {}", entry.seq, e))?;
        // Record the verified entry locally (re-persisting the authority's
        // signature as received).
        let line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| e.to_string())?;
        writeln!(file, "{}", line).map_err(|e| e.to_string())?;
        log.entries.push(entry);
        applied += 1;
    }
    persistence::save_state(&sim, state_path).map_err(|e| format!("{}", e))?;

    // Cross-check state hashes with the sequencer.
    let status_body = ureq::get(&format!("{}/status", sequencer))
        .call()
        .map_err(|e| format!("cannot fetch status: {}", e))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let status: serde_json::Value =
        serde_json::from_str(&status_body).map_err(|e| e.to_string())?;
    let local_hash = txapply::state_hash(&sim);
    let remote_hash = status["state_hash"].as_str().unwrap_or("");
    let in_sync = log.next_seq() == status["seq"].as_u64().unwrap_or(0);
    if in_sync && local_hash != remote_hash {
        return Err(format!(
            "STATE DIVERGENCE at seq {}: local {} vs sequencer {}",
            log.next_seq(),
            local_hash,
            remote_hash
        ));
    }

    Ok(serde_json::json!({
        "applied": applied,
        "seq": log.next_seq(),
        "state_hash": local_hash,
        "in_sync": in_sync,
    }))
}

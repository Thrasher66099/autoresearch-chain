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

//! Ed25519 identity for AutoResearch Chain (Milestone E1).
//!
//! A participant's identity **is** their Ed25519 public key: the 32-byte
//! `ParticipantId` / `ValidatorId` / `ProposerId` on protocol objects is
//! the raw public key, so any submission is verifiable against the actor
//! field itself — no registry lookup required.
//!
//! # Signing messages
//!
//! Signatures are computed over deterministic, domain-separated message
//! strings built from selected transaction fields — never over serialized
//! JSON, whose float formatting differs across languages. Fields are
//! joined with `|` after a versioned type tag. IDs and hashes appear as
//! 64-char lowercase hex; integers in decimal; floats as the big-endian
//! hex of their IEEE-754 f64 bits (`f64::to_bits`); absent optionals as
//! the literal `none`. The Python builder in `arc_runner.identity` must
//! produce byte-identical messages.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Errors from signature verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityError {
    /// The actor field is not a valid Ed25519 public key.
    InvalidPublicKey,
    /// The signature bytes are malformed.
    MalformedSignature,
    /// The signature does not verify against the message and key.
    BadSignature,
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPublicKey => write!(f, "actor id is not a valid Ed25519 public key"),
            Self::MalformedSignature => write!(f, "malformed signature (expected 64 bytes)"),
            Self::BadSignature => write!(f, "signature verification failed"),
        }
    }
}

/// An Ed25519 keypair. The public key doubles as the participant ID.
pub struct Keypair {
    signing: SigningKey,
}

impl Keypair {
    /// Generate a fresh keypair from OS randomness.
    pub fn generate() -> Self {
        let mut secret = [0u8; 32];
        getrandom::getrandom(&mut secret).expect("OS randomness unavailable");
        Self::from_secret_bytes(&secret)
    }

    /// Deterministic keypair from 32 secret bytes.
    pub fn from_secret_bytes(secret: &[u8; 32]) -> Self {
        Self { signing: SigningKey::from_bytes(secret) }
    }

    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }

    /// The public key — also the participant ID bytes.
    pub fn public_bytes(&self) -> [u8; 32] {
        self.signing.verifying_key().to_bytes()
    }

    /// Sign a message, returning the 64-byte signature.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }
}

/// Verify a signature against a 32-byte public key (the actor ID).
pub fn verify(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8],
) -> Result<(), IdentityError> {
    let key = VerifyingKey::from_bytes(public_key)
        .map_err(|_| IdentityError::InvalidPublicKey)?;
    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| IdentityError::MalformedSignature)?;
    let sig = Signature::from_bytes(&sig_bytes);
    key.verify(message, &sig)
        .map_err(|_| IdentityError::BadSignature)
}

fn f64_bits_hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn opt_f64(x: Option<f64>) -> String {
    x.map(f64_bits_hex).unwrap_or_else(|| "none".to_string())
}

/// Signing message for a block submission.
#[allow(clippy::too_many_arguments)]
pub fn block_message(
    id: &str,
    domain_id: &str,
    parent_id: &str,
    proposer: &str,
    child_state_ref: &str,
    diff_ref: &str,
    claimed_metric_delta: f64,
    evidence_bundle_hash: &str,
    fee: u64,
    bond: u64,
    epoch_id: u64,
    timestamp: u64,
) -> Vec<u8> {
    format!(
        "arc-block-v1|{id}|{domain_id}|{parent_id}|{proposer}|{child_state_ref}|{diff_ref}|{}|{evidence_bundle_hash}|{fee}|{bond}|{epoch_id}|{timestamp}",
        f64_bits_hex(claimed_metric_delta),
    )
    .into_bytes()
}

/// Signing message for a validation attestation.
pub fn attestation_message(
    block_id: &str,
    validator: &str,
    vote: &str,
    observed_delta: Option<f64>,
    replay_evidence_ref: &str,
    timestamp: u64,
) -> Vec<u8> {
    format!(
        "arc-attestation-v1|{block_id}|{validator}|{vote}|{}|{replay_evidence_ref}|{timestamp}",
        opt_f64(observed_delta),
    )
    .into_bytes()
}

/// Signing message for a genesis submission. The genesis ID is a
/// content-derived commitment, so signing it covers the full package.
pub fn genesis_message(id: &str, proposer: &str, timestamp: u64) -> Vec<u8> {
    format!("arc-genesis-v1|{id}|{proposer}|{timestamp}").into_bytes()
}

/// Signing message for opening a challenge.
pub fn challenge_message(
    challenge_id: &str,
    challenge_type: &str,
    target_block_id: &str,
    challenger: &str,
    bond: u64,
    evidence_ref: &str,
) -> Vec<u8> {
    format!(
        "arc-challenge-v1|{challenge_id}|{challenge_type}|{target_block_id}|{challenger}|{bond}|{evidence_ref}"
    )
    .into_bytes()
}

/// Signing message for a seed validation record.
pub fn seed_validation_message(
    genesis_id: &str,
    validator: &str,
    vote: &str,
    observed_score: Option<f64>,
    timestamp: u64,
) -> Vec<u8> {
    format!(
        "arc-seedval-v1|{genesis_id}|{validator}|{vote}|{}|{timestamp}",
        opt_f64(observed_score),
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_keypair() -> Keypair {
        Keypair::from_secret_bytes(&[1u8; 32])
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let kp = fixed_keypair();
        let msg = block_message(
            "aa", "bb", "cc", "dd", "ee", "ff", 0.015, "11", 10, 500, 1, 1_700_000_000,
        );
        let sig = kp.sign(&msg);
        verify(&kp.public_bytes(), &msg, &sig).unwrap();
    }

    #[test]
    fn tampered_message_fails() {
        let kp = fixed_keypair();
        let sig = kp.sign(b"hello");
        assert_eq!(
            verify(&kp.public_bytes(), b"hellx", &sig),
            Err(IdentityError::BadSignature)
        );
    }

    #[test]
    fn wrong_key_fails() {
        let kp = fixed_keypair();
        let other = Keypair::from_secret_bytes(&[2u8; 32]);
        let sig = kp.sign(b"hello");
        assert_eq!(
            verify(&other.public_bytes(), b"hello", &sig),
            Err(IdentityError::BadSignature)
        );
    }

    #[test]
    fn message_format_is_pinned() {
        // Cross-language stability: the Python builder must match this
        // byte-for-byte. 0.015f64 bits = 0x3f8eb851eb851eb8.
        let msg = attestation_message("b1", "v1", "Pass", Some(0.015), "r1", 5);
        assert_eq!(
            String::from_utf8(msg).unwrap(),
            "arc-attestation-v1|b1|v1|Pass|3f8eb851eb851eb8|r1|5"
        );
        let msg = attestation_message("b1", "v1", "Fail", None, "r1", 5);
        assert_eq!(
            String::from_utf8(msg).unwrap(),
            "arc-attestation-v1|b1|v1|Fail|none|r1|5"
        );
    }

    #[test]
    fn keypair_generation_produces_valid_identity() {
        let kp = Keypair::generate();
        let sig = kp.sign(b"x");
        verify(&kp.public_bytes(), b"x", &sig).unwrap();
        // Identity is the public key: 32 bytes, usable as a ParticipantId.
        assert_eq!(kp.public_bytes().len(), 32);
    }
}

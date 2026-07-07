// SPDX-License-Identifier: AGPL-3.0-or-later

//! Configuration for protocol rules thresholds.

use serde::{Serialize, Deserialize};

/// Configuration governing block validation and acceptance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Number of validators to assign per block.
    pub validators_per_block: usize,
    /// Minimum Pass votes required for provisional acceptance.
    pub acceptance_quorum: u32,
    /// Maximum Fail votes before rejection.
    pub rejection_threshold: u32,
    /// Maximum Inconclusive votes before marking inconclusive.
    pub inconclusive_threshold: u32,
    /// Any FraudSuspected vote triggers rejection.
    pub fraud_triggers_rejection: bool,
    /// Minimum bond required to submit a block.
    pub min_block_bond: u64,
    /// Whether an Inconclusive provisional outcome causes rejection.
    /// When false, Inconclusive blocks remain in a pending state.
    pub inconclusive_is_rejection: bool,
    /// Minimum validated improvement (in the metric's improvement
    /// direction) required for acceptance. Claims inside the attestation
    /// tolerance band are unfalsifiable by replay, so improvements below
    /// noise scale must earn nothing — otherwise noise mining farms block
    /// rewards risk-free (adversarial-sim finding). Must be calibrated
    /// above the replay tolerance band. Zero disables the check.
    pub min_accepted_delta: f64,
    /// Slashable bond each validator posts at registration. Zero
    /// disables validator bonding (legacy/test states). Required for
    /// attestation slashing to bite.
    pub validator_bond: u64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validators_per_block: 3,
            acceptance_quorum: 2,
            rejection_threshold: 2,
            inconclusive_threshold: 2,
            fraud_triggers_rejection: true,
            min_block_bond: 50,
            inconclusive_is_rejection: true,
            min_accepted_delta: 0.0,
            validator_bond: 0,
        }
    }
}

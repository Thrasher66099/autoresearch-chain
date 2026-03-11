// SPDX-License-Identifier: AGPL-3.0-or-later

//! Configuration for domain-engine policy thresholds.
//!
//! Separated from logic so thresholds can be tuned without touching
//! state machine code.

/// Configuration governing genesis activation thresholds.
#[derive(Clone, Debug)]
pub struct GenesisActivationConfig {
    /// Minimum number of seed validation attestations required.
    pub min_seed_validations: u32,
    /// Minimum fraction of Pass votes (0.0 to 1.0) for seed acceptance.
    pub seed_pass_threshold: f64,
    /// Minimum bond required to propose a genesis block.
    pub min_genesis_bond: u64,
    /// Whether any FraudSuspected vote triggers activation failure.
    pub fraud_triggers_failure: bool,
}

impl Default for GenesisActivationConfig {
    fn default() -> Self {
        Self {
            min_seed_validations: 3,
            seed_pass_threshold: 0.67,
            min_genesis_bond: 100,
            fraud_triggers_failure: true,
        }
    }
}

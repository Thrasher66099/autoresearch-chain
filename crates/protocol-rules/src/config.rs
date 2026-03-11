// SPDX-License-Identifier: AGPL-3.0-or-later

//! Configuration for protocol rules thresholds.

/// Configuration governing block validation and acceptance.
#[derive(Clone, Debug)]
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
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Attestation aggregation and provisional acceptance logic.

use arc_protocol_types::{MetricValue, ValidationAttestation, ValidatorVote};
use crate::config::ValidationConfig;

/// Aggregated summary of attestations for a single block.
#[derive(Clone, Debug)]
pub struct AttestationSummary {
    pub total: u32,
    pub pass_count: u32,
    pub fail_count: u32,
    pub inconclusive_count: u32,
    pub fraud_count: u32,
    /// Mean of observed deltas from Pass votes (if any).
    pub mean_observed_delta: Option<f64>,
    /// Min observed delta from Pass votes.
    pub min_observed_delta: Option<f64>,
    /// Max observed delta from Pass votes.
    pub max_observed_delta: Option<f64>,
}

/// The provisional outcome after evaluating attestations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvisionalOutcome {
    /// Block accepted: sufficient Pass votes, no fraud.
    Accepted,
    /// Block rejected: too many Fail/FraudSuspected votes.
    Rejected,
    /// Not enough information to decide.
    Inconclusive,
}

/// Aggregate attestations into a summary.
pub fn aggregate_attestations(attestations: &[ValidationAttestation]) -> AttestationSummary {
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut inconclusive = 0u32;
    let mut fraud = 0u32;
    let mut deltas: Vec<f64> = Vec::new();

    for att in attestations {
        match att.vote {
            ValidatorVote::Pass => {
                pass += 1;
                if let Some(delta) = att.observed_delta {
                    deltas.push(delta.as_f64());
                }
            }
            ValidatorVote::Fail => fail += 1,
            ValidatorVote::Inconclusive => inconclusive += 1,
            ValidatorVote::FraudSuspected => fraud += 1,
        }
    }

    let (mean, min, max) = if deltas.is_empty() {
        (None, None, None)
    } else {
        let sum: f64 = deltas.iter().sum();
        let mean = sum / deltas.len() as f64;
        let min = deltas.iter().copied().fold(f64::INFINITY, f64::min);
        let max = deltas.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (Some(mean), Some(min), Some(max))
    };

    AttestationSummary {
        total: attestations.len() as u32,
        pass_count: pass,
        fail_count: fail,
        inconclusive_count: inconclusive,
        fraud_count: fraud,
        mean_observed_delta: mean,
        min_observed_delta: min,
        max_observed_delta: max,
    }
}

/// Evaluate a block's attestation summary against config thresholds.
///
/// Returns the provisional outcome.
pub fn evaluate_provisional_outcome(
    summary: &AttestationSummary,
    config: &ValidationConfig,
) -> ProvisionalOutcome {
    // Fraud always triggers rejection.
    if config.fraud_triggers_rejection && summary.fraud_count > 0 {
        return ProvisionalOutcome::Rejected;
    }

    // Rejection threshold.
    if summary.fail_count >= config.rejection_threshold {
        return ProvisionalOutcome::Rejected;
    }

    // Acceptance quorum.
    if summary.pass_count >= config.acceptance_quorum {
        return ProvisionalOutcome::Accepted;
    }

    // If too many inconclusive, mark inconclusive.
    if summary.inconclusive_count >= config.inconclusive_threshold {
        return ProvisionalOutcome::Inconclusive;
    }

    // Default: not enough data yet.
    ProvisionalOutcome::Inconclusive
}

/// Validate that the observed deltas are within tolerance of the
/// claimed delta. Returns true if the mean delta is within tolerance.
///
/// Phase 0.2: simple absolute-difference check.
pub fn deltas_within_tolerance(
    summary: &AttestationSummary,
    claimed_delta: MetricValue,
    tolerance: MetricValue,
) -> bool {
    match summary.mean_observed_delta {
        Some(mean) => (mean - claimed_delta.as_f64()).abs() <= tolerance.as_f64(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arc_protocol_types::fixtures::{test_block_id, test_validator_id, test_artifact_hash};

    fn make_attestation(vote: ValidatorVote, delta: Option<f64>) -> ValidationAttestation {
        ValidationAttestation {
            block_id: test_block_id(1),
            validator: test_validator_id(1),
            vote,
            observed_delta: delta.map(MetricValue::new),
            replay_evidence_ref: test_artifact_hash(1),
            timestamp: 1000,
        }
    }

    #[test]
    fn aggregate_all_pass() {
        let atts = vec![
            make_attestation(ValidatorVote::Pass, Some(0.01)),
            make_attestation(ValidatorVote::Pass, Some(0.02)),
            make_attestation(ValidatorVote::Pass, Some(0.03)),
        ];
        let summary = aggregate_attestations(&atts);
        assert_eq!(summary.pass_count, 3);
        assert_eq!(summary.fail_count, 0);
        assert!(summary.mean_observed_delta.is_some());
        assert!((summary.mean_observed_delta.unwrap() - 0.02).abs() < 1e-10);
    }

    #[test]
    fn aggregate_mixed_votes() {
        let atts = vec![
            make_attestation(ValidatorVote::Pass, Some(0.01)),
            make_attestation(ValidatorVote::Fail, None),
            make_attestation(ValidatorVote::Inconclusive, None),
        ];
        let summary = aggregate_attestations(&atts);
        assert_eq!(summary.pass_count, 1);
        assert_eq!(summary.fail_count, 1);
        assert_eq!(summary.inconclusive_count, 1);
    }

    #[test]
    fn provisional_acceptance() {
        let config = ValidationConfig::default();
        let summary = AttestationSummary {
            total: 3,
            pass_count: 2,
            fail_count: 0,
            inconclusive_count: 1,
            fraud_count: 0,
            mean_observed_delta: Some(0.015),
            min_observed_delta: Some(0.01),
            max_observed_delta: Some(0.02),
        };
        assert_eq!(evaluate_provisional_outcome(&summary, &config), ProvisionalOutcome::Accepted);
    }

    #[test]
    fn provisional_rejection_by_fail() {
        let config = ValidationConfig::default();
        let summary = AttestationSummary {
            total: 3,
            pass_count: 1,
            fail_count: 2,
            inconclusive_count: 0,
            fraud_count: 0,
            mean_observed_delta: None,
            min_observed_delta: None,
            max_observed_delta: None,
        };
        assert_eq!(evaluate_provisional_outcome(&summary, &config), ProvisionalOutcome::Rejected);
    }

    #[test]
    fn provisional_rejection_by_fraud() {
        let config = ValidationConfig::default();
        let summary = AttestationSummary {
            total: 3,
            pass_count: 2,
            fail_count: 0,
            inconclusive_count: 0,
            fraud_count: 1,
            mean_observed_delta: Some(0.015),
            min_observed_delta: Some(0.01),
            max_observed_delta: Some(0.02),
        };
        assert_eq!(evaluate_provisional_outcome(&summary, &config), ProvisionalOutcome::Rejected);
    }

    #[test]
    fn provisional_inconclusive() {
        let config = ValidationConfig::default();
        let summary = AttestationSummary {
            total: 3,
            pass_count: 1,
            fail_count: 0,
            inconclusive_count: 2,
            fraud_count: 0,
            mean_observed_delta: None,
            min_observed_delta: None,
            max_observed_delta: None,
        };
        assert_eq!(evaluate_provisional_outcome(&summary, &config), ProvisionalOutcome::Inconclusive);
    }

    #[test]
    fn delta_tolerance_check() {
        let summary = AttestationSummary {
            total: 3,
            pass_count: 3,
            fail_count: 0,
            inconclusive_count: 0,
            fraud_count: 0,
            mean_observed_delta: Some(0.014),
            min_observed_delta: Some(0.012),
            max_observed_delta: Some(0.016),
        };
        assert!(deltas_within_tolerance(
            &summary,
            MetricValue::new(0.015),
            MetricValue::new(0.005),
        ));
        assert!(!deltas_within_tolerance(
            &summary,
            MetricValue::new(0.015),
            MetricValue::new(0.0005),
        ));
    }
}

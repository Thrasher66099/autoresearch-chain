// SPDX-License-Identifier: AGPL-3.0-or-later

//! Research Track Standard conformance checking.
//!
//! RTS conformance checks are distinct from structural validation.
//! Structural validation (in `protocol-types/validate.rs`) ensures the
//! genesis block data is well-formed. RTS conformance checks ensure the
//! genesis declares parameters consistent with the semantics of the
//! claimed RTS version.
//!
//! Callers must run structural validation first. RTS conformance
//! assumes structurally valid input.
//!
//! RTS-1 is the only standard defined for Stage 1:
//! single-metric, fixed-budget, bounded single-node replay.

use arc_protocol_types::{GenesisBlock, ResearchTrackStandardVersion};

/// Check RTS conformance for a genesis block.
///
/// Returns Ok(()) if the genesis block conforms, or a list of reasons
/// it does not.
pub fn check_rts_conformance(genesis: &GenesisBlock) -> Result<(), Vec<String>> {
    match genesis.rts_version {
        ResearchTrackStandardVersion::Rts1 => check_rts1_conformance(genesis),
    }
}

/// RTS-1 conformance: single-metric, fixed-budget, bounded replay.
///
/// Structural well-formedness (non-empty fields, non-zero hashes, etc.)
/// is assumed to have been checked already. This function checks RTS-1
/// semantic constraints only.
fn check_rts1_conformance(genesis: &GenesisBlock) -> Result<(), Vec<String>> {
    let mut reasons = Vec::new();

    // RTS-1: time budget must be bounded to single-node replay scale.
    // Upper bound is a policy parameter; for Phase 0.2 we check a
    // reasonable structural limit.
    if genesis.time_budget_secs > 86400 {
        reasons.push(
            "RTS-1 requires time_budget_secs <= 86400 (single-node replay bound)".to_string(),
        );
    }

    // RTS-1: search surface and frozen surface must not overlap.
    // (This is also checked structurally, but its semantic meaning for
    // RTS-1 is that the experiment partitioning is valid.)
    // The structural check in validate.rs already covers this, so we
    // rely on that and do not re-check here.

    // RTS-1: secondary metrics are not supported (single-metric standard).
    // Currently the genesis type doesn't have a secondary_metrics field,
    // but when it does, RTS-1 should reject non-empty secondary metrics.

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arc_protocol_types::fixtures;

    #[test]
    fn valid_genesis_passes_rts1() {
        let g = fixtures::valid_genesis_block();
        assert!(check_rts_conformance(&g).is_ok());
    }

    #[test]
    fn rts1_rejects_excessive_time_budget() {
        let mut g = fixtures::valid_genesis_block();
        g.time_budget_secs = 100_000;
        let err = check_rts_conformance(&g).unwrap_err();
        assert!(err.iter().any(|r| r.contains("time_budget")));
    }
}

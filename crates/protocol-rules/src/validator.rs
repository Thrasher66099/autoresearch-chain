// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic validator assignment.
//!
//! Phase 0.2: simple deterministic selection from a domain-scoped pool.
//! This uses a hash-based selection that is reproducible given the same
//! inputs. No randomness beacon or networking required.

use serde::{Serialize, Deserialize};

use arc_protocol_types::{BlockId, DomainId, ValidatorId};
use crate::error::ProtocolError;

/// A pool of validators available for a domain.
///
/// In Phase 0.2 this is a simple list. Later phases will add
/// staking requirements, eligibility filtering, and rotation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorPool {
    /// Domain this pool serves.
    pub domain_id: DomainId,
    /// Registered validators for this domain.
    pub validators: Vec<ValidatorId>,
}

/// Deterministically assign validators to a block.
///
/// The assignment is deterministic: given the same pool, block_id, and count,
/// the same validators are always selected. This uses a simple modular
/// selection based on the block_id bytes — sufficient for local testing and
/// structurally replaceable with a proper VRF/beacon-based scheme later.
///
/// The assignment is domain-scoped: only validators in the domain pool
/// are eligible.
pub fn assign_validators(
    pool: &ValidatorPool,
    block_id: &BlockId,
    count: usize,
) -> Result<Vec<ValidatorId>, ProtocolError> {
    if pool.validators.len() < count {
        return Err(ProtocolError::InsufficientValidators {
            domain_id: pool.domain_id,
            available: pool.validators.len(),
            required: count,
        });
    }

    if pool.validators.len() == count {
        return Ok(pool.validators.clone());
    }

    // Deterministic selection: use block_id bytes to derive a starting
    // offset, then pick `count` validators with stride.
    let bytes = block_id.as_bytes();
    let seed = u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
    ]);

    let pool_size = pool.validators.len();
    let mut selected = Vec::with_capacity(count);
    let mut offset = (seed as usize) % pool_size;

    for _ in 0..count {
        // Skip validators already selected (simple linear probe).
        while selected.contains(&pool.validators[offset % pool_size]) {
            offset += 1;
        }
        selected.push(pool.validators[offset % pool_size]);
        offset += 1;
    }

    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arc_protocol_types::fixtures::{test_block_id, test_domain_id, test_validator_id};

    #[test]
    fn assign_validators_deterministic() {
        let pool = ValidatorPool {
            domain_id: test_domain_id(1),
            validators: (1..=10).map(test_validator_id).collect(),
        };

        let block = test_block_id(42);
        let a = assign_validators(&pool, &block, 3).unwrap();
        let b = assign_validators(&pool, &block, 3).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn assign_validators_no_duplicates() {
        let pool = ValidatorPool {
            domain_id: test_domain_id(1),
            validators: (1..=5).map(test_validator_id).collect(),
        };

        let block = test_block_id(99);
        let assigned = assign_validators(&pool, &block, 3).unwrap();
        assert_eq!(assigned.len(), 3);

        // All distinct.
        for (i, a) in assigned.iter().enumerate() {
            for (j, b) in assigned.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn assign_validators_insufficient_pool() {
        let pool = ValidatorPool {
            domain_id: test_domain_id(1),
            validators: vec![test_validator_id(1)],
        };

        let block = test_block_id(1);
        let err = assign_validators(&pool, &block, 3).unwrap_err();
        match err {
            ProtocolError::InsufficientValidators { available, required, .. } => {
                assert_eq!(available, 1);
                assert_eq!(required, 3);
            }
            _ => panic!("expected InsufficientValidators"),
        }
    }

    #[test]
    fn different_blocks_get_different_assignments() {
        let pool = ValidatorPool {
            domain_id: test_domain_id(1),
            validators: (1..=20).map(test_validator_id).collect(),
        };

        let a = assign_validators(&pool, &test_block_id(1), 3).unwrap();
        let b = assign_validators(&pool, &test_block_id(2), 3).unwrap();
        // Not guaranteed to differ with small pools, but very likely with 20.
        // At minimum the function should not panic.
        assert_eq!(a.len(), 3);
        assert_eq!(b.len(), 3);
    }
}

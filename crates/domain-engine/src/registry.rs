// SPDX-License-Identifier: AGPL-3.0-or-later

//! Domain registry — tracks active domains and their state.

use std::collections::HashMap;

use arc_protocol_types::{
    DomainId, DomainSpec, GenesisBlockId, ProblemDomain, TrackTree,
};

use crate::error::DomainError;
use crate::genesis::ActivatedDomain;

/// Local registry of active domains and their associated objects.
///
/// This is the in-memory state store for Phase 0.2. It holds domains,
/// specs, and track trees. A future phase will replace this with
/// persistent storage.
#[derive(Clone, Debug, Default)]
pub struct DomainRegistry {
    pub domains: HashMap<DomainId, ProblemDomain>,
    pub specs: HashMap<DomainId, DomainSpec>,
    pub track_trees: HashMap<GenesisBlockId, TrackTree>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an activated domain. Fails if the domain already exists.
    pub fn register(&mut self, activated: ActivatedDomain) -> Result<(), DomainError> {
        let domain_id = activated.domain.id;

        if self.domains.contains_key(&domain_id) {
            return Err(DomainError::DomainAlreadyExists { domain_id });
        }

        let genesis_id = activated.track_tree.genesis_block_id;
        self.domains.insert(domain_id, activated.domain);
        self.specs.insert(domain_id, activated.domain_spec);
        self.track_trees.insert(genesis_id, activated.track_tree);

        Ok(())
    }

    /// Check whether a domain is registered and active.
    pub fn is_active(&self, domain_id: &DomainId) -> bool {
        self.domains.contains_key(domain_id)
    }

    /// Get the track tree for a genesis block.
    pub fn get_track_tree(&self, genesis_id: &GenesisBlockId) -> Option<&TrackTree> {
        self.track_trees.get(genesis_id)
    }

    /// Get a mutable reference to a track tree.
    pub fn get_track_tree_mut(&mut self, genesis_id: &GenesisBlockId) -> Option<&mut TrackTree> {
        self.track_trees.get_mut(genesis_id)
    }

    /// Get the domain spec for a domain.
    pub fn get_spec(&self, domain_id: &DomainId) -> Option<&DomainSpec> {
        self.specs.get(domain_id)
    }
}

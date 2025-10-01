//! Tableau edge management
//!
//! This module handles edges between tableau nodes, representing
//! role relationships in the tableau.

use super::node::{NodeId, RoleLabel};
use crate::core::dependency::DependencySet;

/// Edge between tableau nodes
#[derive(Debug, Clone)]
pub struct TableauEdge {
    /// Source node
    pub from: NodeId,

    /// Target node
    pub to: NodeId,

    /// Role label
    pub role: RoleLabel,

    /// Dependency information
    pub dependencies: DependencySet,
}

impl TableauEdge {
    /// Create a new tableau edge
    pub fn new(from: NodeId, to: NodeId, role: RoleLabel, dependencies: DependencySet) -> Self {
        Self {
            from,
            to,
            role,
            dependencies,
        }
    }

    /// Get the role name
    pub fn role_name(&self) -> &str {
        self.role.name()
    }

    /// Check if this edge represents an inverse role
    pub fn is_inverse(&self) -> bool {
        matches!(self.role, RoleLabel::Inverse(_))
    }

    /// Create the inverse of this edge
    pub fn inverse(&self) -> Self {
        let inverse_role = match &self.role {
            RoleLabel::Atomic(name) => RoleLabel::Inverse(name.clone()),
            RoleLabel::Inverse(name) => RoleLabel::Atomic(name.clone()),
            RoleLabel::Chain(_) => self.role.clone(), // Chains don't have simple inverses
            RoleLabel::Complex(_) => self.role.clone(), // Complex roles keep their form
        };

        Self {
            from: self.to,
            to: self.from,
            role: inverse_role,
            dependencies: self.dependencies.clone(),
        }
    }
}

/// Property inclusion relationship (`SubObjectPropertyOf`)
#[derive(Debug, Clone)]
pub struct PropertyInclusion {
    /// Subproperty
    pub sub_property: RoleLabel,

    /// Superproperty
    pub super_property: RoleLabel,

    /// Dependencies for this inclusion
    pub dependencies: DependencySet,
}

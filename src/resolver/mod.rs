//! Dependency resolution surface for `semverx`.
//!
//! Exposes:
//! - [`DependencyResolver`] — trait implemented by concrete resolvers
//! - [`Component`] — a node in the dependency graph
//! - [`ResolutionError`] — errors emitted during resolution
//! - [`graph`] — A*/Eulerian/Hamiltonian graph-based resolver
//!
//! The SemverX topology tri-model (Eulerian visits all edges, Hamiltonian
//! visits all nodes, A* finds the nearest viable path) is encoded in the
//! `graph` submodule.

pub mod graph;

pub use graph::{GraphResolver, SemverXResolver};

use std::fmt;

/// A resolvable component in the dependency graph.
///
/// Kept intentionally small so it can serve as a `petgraph` node weight. For
/// the richer component schema (SEI metadata, stress zones, verb-noun class),
/// see `resolver::types::Component`, which will supersede this once the
/// surrounding SemverX type system lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// Package name, e.g. `lodash`.
    pub name: String,
    /// Package version as a raw semver string, e.g. `1.2.3`.
    pub version: String,
    /// Names of packages this component depends on.
    pub dependencies: Vec<String>,
}

impl Component {
    /// Build a new [`Component`] with no dependencies.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            dependencies: Vec::new(),
        }
    }
}

/// Errors surfaced by a [`DependencyResolver`].
#[derive(Debug, Clone)]
pub enum ResolutionError {
    /// Requested component id was not present in the graph.
    ComponentNotFound(String),
    /// Two or more constraints could not be satisfied simultaneously.
    VersionConflict(String),
    /// A cycle was detected while walking the graph.
    CyclicDependency(String),
    /// The resolver exhausted its iteration budget.
    MaxIterationsExceeded,
    /// The underlying graph failed an invariant check.
    GraphValidationFailed(String),
    /// No path exists between two nodes (from, to).
    NoPathFound(String, String),
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolutionError::ComponentNotFound(id) => {
                write!(f, "Component not found: {}", id)
            }
            ResolutionError::VersionConflict(msg) => {
                write!(f, "Version conflict: {}", msg)
            }
            ResolutionError::CyclicDependency(msg) => {
                write!(f, "Cyclic dependency detected: {}", msg)
            }
            ResolutionError::MaxIterationsExceeded => {
                write!(f, "Maximum resolution iterations exceeded")
            }
            ResolutionError::GraphValidationFailed(msg) => {
                write!(f, "Graph validation failed: {}", msg)
            }
            ResolutionError::NoPathFound(from, to) => {
                write!(f, "No path from {} to {}", from, to)
            }
        }
    }
}

impl std::error::Error for ResolutionError {}

/// Contract implemented by any concrete SemverX resolver.
///
/// Keeping this a trait (rather than a concrete struct) lets us plug in
/// Eulerian, Hamiltonian, A*, or hybrid strategies behind the same surface —
/// which is exactly what the registry's `.monoglot` / `.polyglot` / `.hybrid`
/// schemas ask for.
pub trait DependencyResolver {
    /// Resolve the full dependency closure for `package` at `version`.
    fn resolve_dependencies(
        &mut self,
        package: &str,
        version: &str,
    ) -> Result<Vec<Component>, ResolutionError>;

    /// Register a constraint (e.g. `^1.2.0`) against a given package.
    fn add_constraint(&mut self, package: &str, constraint: &str);
}

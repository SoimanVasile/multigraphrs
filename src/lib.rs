//! # MultiGraphRs
//!
//! `multigraphrs` is a versatile and modular graph library built in Rust. 
//! It leverages the **Strategy Pattern** to provide a unified `MultiGraph` data structure 
//! that can behave as a Directed, Undirected, Weighted, or Unweighted graph simply 
//! by swapping out its generic strategy parameter.
//!
//! ## Example
//! ```rust
//! use multigraphrs::{MultiGraph, Directed};
//! 
//! let mut graph = MultiGraph::<&str, u32, Directed>::new();
//! graph.add_node("A").unwrap();
//! graph.add_node("B").unwrap();
//! graph.add_edge("A", "B").unwrap();
//! ```

pub mod edge;
pub mod direction_strategy;
pub mod directed;
pub mod weighted;
pub mod undirected;
pub mod weighted_directed;
pub mod graph_errors;

// Expose the internal types publicly so users can import them easily
pub use direction_strategy::DirectionStrategy;
pub use directed::Directed;
pub use undirected::Undirected;
pub use weighted::Weighted;
pub use weighted_directed::WeightedDirected;
pub use edge::Edge;
pub use graph_errors::GraphErrors;

use std::{collections::HashMap, hash::Hash, marker::PhantomData};

/// The core graph structure representing a mathematical graph.
///
/// `MultiGraph` stores nodes and their corresponding edges in an adjacency list.
/// The specific rules for how edges are added (e.g., directed vs. undirected) 
/// are governed by the generic strategy `S`.
///
/// # Type Parameters
/// * `K`: The type of the nodes (Keys). Must implement `Eq`, `Hash`, and `Clone`.
/// * `W`: The type of the edge weights. Must implement `Clone` (allowing floating-point weights).
/// * `S`: The direction strategy (e.g., `Directed`, `Weighted`).
pub struct MultiGraph<K, W, S: DirectionStrategy<K, W>>
where
    K: Eq + Hash + Clone,
    W: Clone,
{
    /// The internal adjacency list mapping a node to its outgoing edges.
    pub adjacency_list: HashMap<K, Vec<Edge<K, W>>>,
    /// Marker to keep track of the specific strategy `S` being used.
    pub _strategy: PhantomData<S>,
}

// --- Core Methods Shared by ALL Graph Types ---

impl<K, W, S> MultiGraph<K, W, S>
where
    K: Eq + Hash + Clone,
    W: Clone,
    S: DirectionStrategy<K, W>,
{
    /// Adds a single, disconnected node to the graph.
    ///
    /// This is useful for building up the vertices of your graph before 
    /// defining the edges between them.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeAlreadyExists` if the node is already present in the graph.
    pub fn add_node(&mut self, source: K) -> Result<K, GraphErrors> {
        if self.adjacency_list.contains_key(&source) {
            return Err(GraphErrors::NodeAlreadyExists);
        }
        
        self.adjacency_list.entry(source.clone()).or_default();
        Ok(source)
    }
}

// --- Strategy-Specific Implementations ---

impl<K, W> MultiGraph<K, W, Weighted>
where
    K: Eq + Hash + Clone,
    W: Clone,
{
    /// Creates a new, empty `Weighted` (undirected) graph.
    pub fn new() -> MultiGraph<K, W, Weighted> {
        MultiGraph { adjacency_list: HashMap::new(), _strategy: PhantomData }
    }

    /// Adds a weighted edge between two nodes in both directions.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if either the `source` or `target` node 
    /// does not exist in the graph prior to adding the edge.
    pub fn add_edge(&mut self, source: K, target: K, weight: W) -> Result<Vec<Edge<K, W>>, GraphErrors> {
        Weighted::add_edge(&mut self.adjacency_list, &source, &target, &weight)
    }
}

impl<K, W> MultiGraph<K, W, WeightedDirected>
where
    K: Eq + Hash + Clone,
    W: Clone,
{
    /// Creates a new, empty `WeightedDirected` graph.
    pub fn new() -> MultiGraph<K, W, WeightedDirected> {
        MultiGraph { adjacency_list: HashMap::new(), _strategy: PhantomData }
    }

    /// Adds a directed edge from `source` to `target` with the given `weight`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if either node does not exist.
    pub fn add_edge(&mut self, source: K, target: K, weight: W) -> Result<Vec<Edge<K, W>>, GraphErrors> {
        WeightedDirected::add_edge(&mut self.adjacency_list, &source, &target, &weight)
    }
}

impl<K> MultiGraph<K, u32, Directed>
where
    K: Eq + Hash + Clone,
{
    /// Creates a new, empty, unweighted `Directed` graph.
    pub fn new() -> MultiGraph<K, u32, Directed> {
        MultiGraph { adjacency_list: HashMap::new(), _strategy: PhantomData }
    }

    /// Adds a directed edge from `source` to `target` with a default weight of 1.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if either node does not exist.
    pub fn add_edge(&mut self, source: K, target: K) -> Result<Vec<Edge<K, u32>>, GraphErrors> {
        Directed::add_edge(&mut self.adjacency_list, &source, &target, &1)
    }
}

impl<K> MultiGraph<K, u32, Undirected>
where
    K: Eq + Hash + Clone,
{
    /// Creates a new, empty, unweighted `Undirected` graph.
    pub fn new() -> MultiGraph<K, u32, Undirected> {
        MultiGraph { adjacency_list: HashMap::new(), _strategy: PhantomData }
    }

    /// Adds an undirected connection (edges in both directions) between `source` and `target`, defaulting weight to 1.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if either node does not exist.
    pub fn add_edge(&mut self, source: K, target: K) -> Result<Vec<Edge<K, u32>>, GraphErrors> {
        Undirected::add_edge(&mut self.adjacency_list, &source, &target, &1)
    }
}

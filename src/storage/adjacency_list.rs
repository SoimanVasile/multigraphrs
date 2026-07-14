use std::collections::VecDeque;

use crate::core::edge::Edge;
use crate::core::graph_errors::GraphErrors;
use crate::storage::storage_backend::StorageBackend;

/// In-memory graph storage backed by a `Vec<Vec<Edge<W>>>`.
///
/// All data lives in RAM. This is the default backend for `MultiGraph` and
/// is the fastest option for graphs that fit in memory.
pub struct RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    adjacency_list: Vec<Vec<Edge<W>>>,
    /// Tracks incoming edges: reverse_adjacency_list[node] = list of nodes that have edges TO this node.
    /// Used by directed strategies for O(degree) remove_node.
    reverse_adjacency_list: Vec<Vec<u64>>,
    number_of_nodes: usize,
    number_of_edges: usize,
    removed_ids: VecDeque<u64>,
}

impl<W> RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// Creates a new, empty `RamStorage` with no pre-allocated capacity.
    ///
    /// # Returns
    /// An owned, empty `RamStorage` instance.
    ///
    pub fn new() -> RamStorage<W>{
        RamStorage{
            adjacency_list: Vec::new(),
            reverse_adjacency_list: Vec::new(),
            number_of_nodes: 0,
            number_of_edges: 0,
            removed_ids: VecDeque::new(),
        }
    }

    /// Returns an **immutable reference** to the edge vector of `source`.
    ///
    /// The caller borrows from the internal adjacency list; no clone is performed.
    ///
    /// # Panics
    /// Panics if `source` is out of bounds (index >= adjacency_list.len()).
    ///
    pub fn get_edges_ref(&self, source: u64) -> &Vec<Edge<W>>{
        &self.adjacency_list[source as usize]
    }
}

impl<W> StorageBackend<W> for RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    type EdgeIter<'a> = std::vec::IntoIter<Edge<W>> where Self: 'a, W: 'a;

    /// Appends a clone of the given edge to the specified node's adjacency list.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Mutates the adjacency list of the specified node and increments the total edge count.
    fn add_edge_to_node(&mut self, node: &u64, edge: &Edge<W>){
        self.number_of_edges+=1;
        self.adjacency_list[*node as usize].push(edge.clone())
    }

    /// Bulks adds multiple edges to their respective nodes.
    ///
    /// # Panics
    /// Panics if any of the target node indices are out of bounds.
    ///
    /// # Errors
    /// Returns a `std::io::Error` if an underlying I/O error occurs.
    ///
    /// # Side Effects
    /// Mutates the adjacency lists for multiple nodes and updates the total edge count.
    fn bulk_add_edge_to_node(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), std::io::Error> {
        for (source, edge) in edges{
            self.adjacency_list[*source as usize].push(edge.clone());
        }

        Ok(())
    }

    /// Creates a new node in the adjacency list.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Increments the node count. Modifies internal structures by either reusing a freed ID or pushing a new entry.
    fn add_node(&mut self) -> u64{

        self.number_of_nodes+=1;
        if self.removed_ids.is_empty(){
            self.adjacency_list.push(Vec::new());
            self.reverse_adjacency_list.push(Vec::new());

            return (self.number_of_nodes-1) as u64;
        }

        let id = self.removed_ids.pop_front().unwrap();

        id
    }

    /// Bulks adds a specified number of nodes to the adjacency list.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Increments the node count accordingly and adds new entries or reuses freed IDs.
    fn bulk_add_node(&mut self, number_of_nodes: &u64) -> Vec<u64> {
        let mut ids = Vec::with_capacity(*number_of_nodes as usize);
        for _ in 0..*number_of_nodes {
            ids.push(self.add_node());
        }
        ids
    }

    /// Gets the number of edges for a given node.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    fn node_len(&self, node: &u64) -> usize{
        self.adjacency_list[*node as usize].len()
    }

    /// Returns an iterator over the edges of a specific node.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    fn get_edges<'a>(&self, node: &u64) -> Self::EdgeIter<'a> where W: 'a{
        self.adjacency_list[*node as usize].clone().into_iter()
    }

    /// Removes a specific edge from a given node based on a predicate.
    ///
    /// # Panics
    /// Panics if the `source` index is out of bounds.
    ///
    /// # Errors
    /// Returns `GraphErrors::EdgeDoesntExists` if the edge cannot be found.
    ///
    /// # Side Effects
    /// Mutates the adjacency list of the specified node by removing an edge and decrements the total edge count.
    fn remove_edge(&mut self, source: &u64, edge: &Edge<W>) -> Result<Edge<W>, GraphErrors>
    {
        let index = self.adjacency_list[*source as usize]
            .iter()
            .position(|e| e.get_target() == edge.get_target() && e.get_weight() == edge.get_weight());
        if let Some(i) = index {
            self.number_of_edges-=1;
            Ok(self.adjacency_list[*source as usize].swap_remove(i))
        } else {
            Err(GraphErrors::EdgeDoesntExists)
        }
    }

    fn remove_edge_by_property<F>(&mut self, source: &u64, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphErrors>
    where
        F: Fn(&Edge<W>, &Edge<W>) -> bool
    {
        let index = self.adjacency_list[*source as usize]
            .iter()
            .position(|e| func(edge, e));
        if let Some(i) = index {
            self.number_of_edges-=1;
            Ok(self.adjacency_list[*source as usize].swap_remove(i))
        } else {
            Err(GraphErrors::EdgeDoesntExists)
        }
    }

    fn bulk_remove_edge(&mut self, edges: &[(u64, Edge<W>)]) {
        for (source, edge) in edges {
            let _ = self.remove_edge(source, edge);
        }
    }

    /// Searches for an edge between the specified source and target nodes.
    ///
    /// # Panics
    /// Panics if the `source` index is out of bounds.
    ///
    /// # Errors
    /// Returns `GraphErrors::EdgeDoesntExists` if the edge is not present.
    ///
    fn contains_edge(&self, source: &u64, target: &u64) ->Result<Edge<W>, GraphErrors>{
        match self.adjacency_list[*source as usize].iter().position(|e| e.get_target() == *target) {
            Some(t) => Ok(self.adjacency_list[*source as usize][t].clone()),
            None => Err(GraphErrors::EdgeDoesntExists),
        }
    }

    /// Gets the total number of nodes in the graph.
    ///
    /// # Errors
    /// None.
    ///
    fn node_count(&self) -> usize {
        self.number_of_nodes
    }

    /// Gets the total number of edges in the graph.
    ///
    /// # Errors
    /// None.
    ///
    fn edge_count(&self) ->usize{
        self.number_of_edges
    }

    /// Manually increments the internal node counter.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Mutates the internal node count by incrementing it.
    fn increment_node_counter(&mut self) {
        self.number_of_nodes+=1;
    }

    // --- New primitives for strategy-driven remove_node ---

    /// Clears all outgoing edges for a given node.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Empties the adjacency list for the specified node and decrements the global edge count.
    fn clear_node_edges(&mut self, node: &u64) {
        let count = self.adjacency_list[*node as usize].len();
        self.number_of_edges -= count;
        self.adjacency_list[*node as usize].clear();
    }

    /// Removes the first edge from `source` that points to `target`.
    ///
    /// # Panics
    /// Panics if the `source` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Mutates the edge list for `source` and decrements the total edge count.
    fn remove_edge_by_target(&mut self, source: &u64, target: &u64) {
        let list = &mut self.adjacency_list[*source as usize];
        if let Some(pos) = list.iter().position(|e| e.get_target() == *target) {
            list.swap_remove(pos);
            self.number_of_edges -= 1;
        }
    }

    /// Adds an incoming edge record to the reverse adjacency list.
    ///
    /// # Panics
    /// Panics if the `source` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Mutates the reverse adjacency list of `source` by appending `origin`.
    fn add_reverse_edge(&mut self, source: &u64, origin: &u64) {
        self.reverse_adjacency_list[*source as usize].push(*origin);
    }

    /// Bulks adds reverse edge records.
    ///
    /// # Panics
    /// Panics if any of the target indices in the given edges are out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Mutates the reverse adjacency lists for multiple targets.
    fn bulk_add_reverse_edge(&mut self, edges: &[(u64, u64, W)]) {
        for (source, target, _) in edges{
            self.reverse_adjacency_list[*target as usize].push(*source);
        }
    }

    /// Gets all nodes that have an outgoing edge pointing to the specified node.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    fn get_reverse_edges(&self, node: &u64) -> Vec<u64> {
        self.reverse_adjacency_list[*node as usize].clone()
    }

    /// Clears the reverse edge list for a specific node.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Empties the reverse edge list for the specified node.
    fn clear_reverse_edges(&mut self, node: &u64) {
        self.reverse_adjacency_list[*node as usize].clear();
    }

    /// Removes a single reverse edge entry.
    ///
    /// # Panics
    /// Panics if the `source` index is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Mutates the reverse adjacency list of `source` by removing `origin`.
    fn remove_reverse_edge(&mut self, source: &u64, origin: &u64) {
        let list = &mut self.reverse_adjacency_list[*source as usize];
        if let Some(pos) = list.iter().position(|&id| id == *origin) {
            list.swap_remove(pos);
        }
    }

    /// Bulks remove reverse edge entries.
    ///
    /// # Panics
    /// Panics if any of the `source` indices are out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Mutates the reverse adjacency lists of multiple sources.
    fn bulk_remove_reverse_edge(&mut self, edges: &[(u64, u64)]) {
        for (source, origin) in edges {
            self.remove_reverse_edge(source, origin);
        }
    }

    /// Frees a node ID, allowing it to be reused.
    ///
    /// # Panics
    /// Panics if the `node_id` is out of bounds.
    ///
    /// # Errors
    /// None.
    ///
    /// # Side Effects
    /// Decrements the global node count, adds the ID to the removed queue, and clears its adjacency list.
    fn free_node_id(&mut self, node_id: &u64) {
        self.number_of_nodes -= 1;
        self.removed_ids.push_back(*node_id);
        self.adjacency_list[*node_id as usize].clear();
    }
}

impl<W> Default for RamStorage<W>
where
    W: Clone + std::cmp::PartialEq,
{
    /// Creates a default empty `RamStorage`.
    ///
    /// # Errors
    /// None.
    ///
    fn default() -> Self{
        Self::new()
    }
}

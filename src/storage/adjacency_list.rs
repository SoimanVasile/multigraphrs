use crate::dictionary::dictionary_strategy::DictionaryStrategy;
use crate::dictionary::ram_dictionary::RamDictionary;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use std::collections::VecDeque;
use std::hash::Hash;

use crate::core::edge::Edge;
use crate::core::graph_errors::GraphError;
use crate::storage::storage_backend::StorageBackend;

/// In-memory graph storage backed by a `Vec<Vec<Edge<W>>>`.
///
/// All data lives in RAM. This is the default backend for `MultiGraph` and
/// is the fastest option for graphs that fit in memory.
pub struct RamStorage<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    adjacency_list: Vec<Vec<Edge<W>>>,
    /// Tracks incoming edges: reverse_adjacency_list[node] = list of nodes that have edges TO this node.
    /// Used by directed strategies for O(degree) remove_node.
    reverse_adjacency_list: Vec<Vec<u64>>,
    number_of_nodes: usize,
    number_of_edges: usize,
    removed_ids: VecDeque<u64>,
    hashed_nodes: RamDictionary<K>,
}

impl<K, W> RamStorage<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    /// Creates a new, empty `RamStorage` with no pre-allocated capacity.
    ///
    /// # Returns
    /// An owned, empty `RamStorage` instance.
    ///
    pub fn new() -> Self {
        Self {
            adjacency_list: Vec::new(),
            reverse_adjacency_list: Vec::new(),
            number_of_nodes: 0,
            number_of_edges: 0,
            removed_ids: VecDeque::new(),
            hashed_nodes: RamDictionary::new(),
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

impl<K, W> StorageBackend<K, W> for RamStorage<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + AsDiskBytes + FromDiskBytes + Hash,
{
    type EdgeIter<'a> = std::vec::IntoIter<Edge<W>> where Self: 'a, W: 'a;

    /// Appends a clone of the given edge to the specified node's adjacency list.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    /// # Side Effects
    /// Mutates the adjacency list of the specified node and increments the total edge count.
    fn add_edge_to_node(&mut self, node: &u64, edge: &Edge<W>) -> Result<(), GraphError> {
        self.number_of_edges+=1;
        self.adjacency_list[*node as usize].push(edge.clone());
        Ok(())
    }

    /// Bulks adds multiple edges to their respective nodes.
    ///
    /// # Panics
    /// Panics if any of the target node indices are out of bounds.
    ///
    /// # Side Effects
    /// Mutates the adjacency lists for multiple nodes and updates the total edge count.
    fn bulk_add_edge_to_node(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), GraphError> {
        for (source, edge) in edges{
            self.adjacency_list[*source as usize].push(edge.clone());
        }
        self.number_of_edges += edges.len();
        Ok(())
    }

    /// Creates a new node in the adjacency list.
    ///
    /// # Side Effects
    /// Increments the node count. Modifies internal structures by either reusing a freed ID or pushing a new entry.
    fn add_node(&mut self) -> Result<u64, GraphError> {

        self.number_of_nodes+=1;
        if self.removed_ids.is_empty(){
            self.adjacency_list.push(Vec::new());
            self.reverse_adjacency_list.push(Vec::new());

            return Ok((self.number_of_nodes-1) as u64);
        }

        let id = self.removed_ids.pop_front().unwrap();

        Ok(id)
    }

    /// Bulks adds a specified number of nodes to the adjacency list.
    ///
    /// # Side Effects
    /// Increments the node count accordingly and adds new entries or reuses freed IDs.
    fn bulk_add_node(&mut self, number_of_nodes: &u64) -> Result<Vec<u64>, GraphError> {
        let mut ids = Vec::with_capacity(*number_of_nodes as usize);
        for _ in 0..*number_of_nodes {
            ids.push(self.add_node()?);
        }
        Ok(ids)
    }

    /// Gets the number of edges for a given node.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    fn node_len(&self, node: &u64) -> usize{
        self.adjacency_list[*node as usize].len()
    }

    /// Returns an iterator over the edges of a specific node.
    ///
    /// # Panics
    /// Panics if the `node` index is out of bounds.
    ///
    fn get_edges<'a>(&self, node: &u64) -> Self::EdgeIter<'a> where W: 'a, K: 'a{
        self.adjacency_list[*node as usize].clone().into_iter()
    }

    /// Removes a specific edge from a given node based on target and weight match.
    ///
    /// # Panics
    /// Panics if the `source` index is out of bounds.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeDoesntExist` if the edge cannot be found.
    ///
    /// # Side Effects
    /// Mutates the adjacency list of the specified node by removing an edge and decrements the total edge count.
    fn remove_edge(&mut self, source: &u64, edge: &Edge<W>) -> Result<Edge<W>, GraphError>
    {
        let index = self.adjacency_list[*source as usize]
            .iter()
            .position(|e| e.get_target() == edge.get_target() && e.get_weight() == edge.get_weight());
        if let Some(i) = index {
            self.number_of_edges-=1;
            Ok(self.adjacency_list[*source as usize].swap_remove(i))
        } else {
            Err(GraphError::EdgeDoesntExist)
        }
    }

    fn remove_edge_by_property<F>(&mut self, source: &u64, edge: &Edge<W>, func: F) -> Result<Edge<W>, GraphError>
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
            Err(GraphError::EdgeDoesntExist)
        }
    }

    fn bulk_remove_edge(&mut self, edges: &[(u64, Edge<W>)]) -> Result<(), GraphError> {
        for (source, edge) in edges {
            let _ = self.remove_edge(source, edge);
        }
        Ok(())
    }

    /// Searches for an edge between the specified source and target nodes.
    ///
    /// # Panics
    /// Panics if the `source` index is out of bounds.
    ///
    /// # Errors
    /// Returns `GraphError::EdgeDoesntExist` if the edge is not present.
    ///
    fn contains_edge(&self, source: &u64, target: &u64) ->Result<Edge<W>, GraphError>{
        match self.adjacency_list[*source as usize].iter().position(|e| e.get_target() == *target) {
            Some(t) => Ok(self.adjacency_list[*source as usize][t].clone()),
            None => Err(GraphError::EdgeDoesntExist),
        }
    }

    fn node_count(&self) -> usize {
        self.number_of_nodes
    }

    fn edge_count(&self) ->usize{
        self.number_of_edges
    }

    fn increment_node_counter(&mut self) -> Result<(), GraphError> {
        self.number_of_nodes+=1;
        Ok(())
    }

    fn clear_node_edges(&mut self, node: &u64) -> Result<(), GraphError> {
        let count = self.adjacency_list[*node as usize].len();
        self.number_of_edges -= count;
        self.adjacency_list[*node as usize].clear();
        Ok(())
    }

    fn remove_edge_by_target(&mut self, source: &u64, target: &u64) -> Result<(), GraphError> {
        let list = &mut self.adjacency_list[*source as usize];
        if let Some(pos) = list.iter().position(|e| e.get_target() == *target) {
            list.swap_remove(pos);
            self.number_of_edges -= 1;
        }
        Ok(())
    }

    fn add_reverse_edge(&mut self, source: &u64, origin: &u64) -> Result<(), GraphError> {
        self.reverse_adjacency_list[*source as usize].push(*origin);
        Ok(())
    }

    fn bulk_add_reverse_edge(&mut self, edges: &[(u64, u64, W)]) -> Result<(), GraphError> {
        for (source, target, _) in edges{
            self.reverse_adjacency_list[*target as usize].push(*source);
        }
        Ok(())
    }

    fn get_reverse_edges(&self, node: &u64) -> Vec<u64> {
        self.reverse_adjacency_list[*node as usize].clone()
    }

    fn clear_reverse_edges(&mut self, node: &u64) -> Result<(), GraphError> {
        self.reverse_adjacency_list[*node as usize].clear();
        Ok(())
    }

    fn remove_reverse_edge(&mut self, source: &u64, origin: &u64) -> Result<(), GraphError> {
        let list = &mut self.reverse_adjacency_list[*source as usize];
        if let Some(pos) = list.iter().position(|&id| id == *origin) {
            list.swap_remove(pos);
        }
        Ok(())
    }

    fn bulk_remove_reverse_edge(&mut self, edges: &[(u64, u64)]) -> Result<(), GraphError> {
        for (source, origin) in edges {
            let _ = self.remove_reverse_edge(source, origin);
        }
        Ok(())
    }

    fn free_node_id(&mut self, node_id: &u64) -> Result<(), GraphError> {
        self.number_of_nodes -= 1;
        self.removed_ids.push_back(*node_id);
        self.adjacency_list[*node_id as usize].clear();
        Ok(())
    }
    
    fn hashed_nodes_contains_key(&self, key: &K) -> Result<bool, GraphError> {
        Ok(self.hashed_nodes.contains_key(key))
    }

    fn hashed_nodes_insert(&mut self, key: K, node_id: u64) -> Result<(), GraphError> {
        Ok(self.hashed_nodes.insert(key, node_id))
    }

    fn hashed_nodes_get(&self,  key: &K) -> Result<Option<u64>, GraphError> {
        Ok(self.hashed_nodes.get(key))
    }

    fn hashed_nodes_remove(&mut self, key: &K) -> Result<Option<u64>, GraphError> {
        Ok(self.hashed_nodes.remove(key))
    }
}

impl<K, W> Default for RamStorage<K, W>
where
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    K: Clone + Eq + Hash + AsDiskBytes + FromDiskBytes,
{
    fn default() -> Self{
        Self::new()
    }
}

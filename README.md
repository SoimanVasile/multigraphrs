# MultiGraphRs

[![Crates.io](https://img.shields.io/crates/v/multigraphrs.svg)](https://crates.io/crates/multigraphrs)
[![Build Status](https://github.com/yourusername/multigraphrs/actions/workflows/rust.yml/badge.svg)](https://github.com/yourusername/multigraphrs/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> ⚠️ **Note: Early Development**
> 
> This project is currently in its early stages of development. The core architecture is functional, but **major breaking changes** to the API may still occur. 
> 
> **Current Roadmap:**
> * **API Simplification**: Modifying the constructor for non-weighted graphs (like `Directed` and `Undirected`) so they only require two generic parameters (Node Type and Strategy) instead of three.
> * **Graph Traversal**: Implementing algorithms like Breadth-First Search (BFS) and Depth-First Search (DFS).
> * **General Improvements**: Adding more convenience methods and making the overall API more user-friendly. 
> 
> Feedback, ideas, and contributions are always welcome!
`multigraphrs` is a versatile, modular, and type-safe graph data structure library for Rust. 

Instead of implementing separate structs for Directed, Undirected, Weighted, and Unweighted graphs, `multigraphrs` leverages the **Strategy Pattern**. A single `MultiGraph` core data structure adapts its behavior at compile-time based on the strategy provided.

## Features

* **Generic Nodes & Weights**: Use any type for nodes (`K: Eq + Hash + Clone`) and any type for weights (`W: Clone`, fully supporting floating-point types like `f64`).
* **Strategy Pattern Architecture**: Cleanly decouples the graph storage (Adjacency List) from the edge insertion logic.
* **Safe Error Handling**: No panics. All graph operations return a custom `Result<T, GraphErrors>` enum.
* **Zero-Cost Abstractions**: Evaluated at compile-time using Rust's traits and `PhantomData`.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
multigraphrs = "0.1.0"
```

## Quick Start

You can easily instantiate entirely different types of graphs just by swapping out the strategy generic parameter. 

```rust
use multigraphrs::{MultiGraph, Directed, Weighted};

fn main() {
    //this creates the MultiGraph that has nodes as str, weights as u32 and is directed
    let mut dir_graph = MultiGraph::<&str, u32, Directed>::new();
    
    //add the nodes in the multigraph
    dir_graph.add_node("New York").unwrap();
    dir_graph.add_node("London").unwrap();
    
    // Directed takes 2 parameters and defaults the weight to 1
    dir_graph.add_edge("New York", "London").unwrap();
    println!("Created a directed edge from New York to London!");

    // Nodes are `u32` (IDs) and weights are `f64` (distances)
    let mut weight_graph = MultiGraph::<u32, f64, Weighted>::new();
    
    weight_graph.add_node(1).unwrap();
    weight_graph.add_node(2).unwrap();
    
    // Weighted takes 3 parameters (source, target, weight)
    let edges = weight_graph.add_edge(1, 2, 305.50).unwrap();
    
    // Automatically creates bi-directional edges (1->2 and 2->1)
    assert_eq!(edges.len(), 2); 
    println!("Created a bi-directional weighted connection!");
}
```

## Available Strategies

The behavior of `MultiGraph` is controlled by these four zero-sized marker structs:

| Strategy | Directed? | Weighted? | `add_edge` Arguments | Edges Created |
| :--- | :--- | :--- | :--- | :--- |
| `Directed` | Yes | No (Defaults to `1`) | `(source, target)` | 1 |
| `Undirected` | No | No (Defaults to `1`) | `(source, target)` | 2 |
| `WeightedDirected` | Yes | Yes | `(source, target, weight)` | 1 |
| `Weighted` | No | Yes | `(source, target, weight)` | 2 |

## Error Handling

All edge and node insertions return a `Result` checking against `GraphErrors`:
* `NodeNotFound`: Triggered if you attempt to create an edge between nodes that haven't been added to the graph yet.
* `NodeAlreadyExists`: Triggered if you try to add a duplicate node.

## License

This project is licensed under the MIT License.

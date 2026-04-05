# MultiGraphRs

[![Crates.io](https://img.shields.io/crates/v/multigraphrs.svg)](https://crates.io/crates/multigraphrs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A **strategy-pattern based** multigraph library for Rust. One generic `MultiGraph<K, W, S>` struct adapts its behavior at compile-time — directed, undirected, weighted, or unweighted — just by swapping the strategy type parameter.

> ⚠️ **Early Development (v0.1)**
>
> The core architecture is functional and tested, but the API surface is still growing. Minor breaking changes may occur before `v1.0`.

## Features

- **Multigraph support** — multiple parallel edges between the same pair of nodes
- **Strategy Pattern** — a single `MultiGraph` core with four interchangeable strategy types
- **Generic nodes & weights** — any `K: Eq + Hash + Clone` for nodes, any `W: Clone + PartialEq` for weights (including `f64`)
- **Safe error handling** — all operations return `Result<T, GraphErrors>`, no panics
- **`EdgeView<K, W>`** — public return type that hides internal indices and exposes user-facing keys
- **Zero-cost abstractions** — strategies are zero-sized types resolved at compile-time via `PhantomData`

## Installation

```toml
[dependencies]
multigraphrs = "0.1.2"
```

## Quick Start

### Unweighted Directed Graph

```rust
use multigraphrs::{MultiGraph, Directed};

let mut graph = MultiGraph::<&str, u32, Directed>::new();

graph.add_node("Berlin").unwrap();
graph.add_node("Paris").unwrap();

// Directed: one-way edge, weight defaults to 1
let edge = graph.add_edge("Berlin", "Paris").unwrap();
assert_eq!(*edge.get_target(), "Paris");
assert_eq!(*edge.get_weight(), 1);

// Parallel edges are allowed (multigraph)
graph.add_edge("Berlin", "Paris").unwrap();
assert_eq!(graph.degree(&"Berlin"), Ok(2));
```

### Unweighted Undirected Graph

```rust
use multigraphrs::{MultiGraph, Undirected};

let mut graph = MultiGraph::<u32, u32, Undirected>::new();

graph.add_node(1).unwrap();
graph.add_node(2).unwrap();

// Undirected: creates edges in both directions
graph.add_edge(1, 2).unwrap();
assert_eq!(graph.degree(&1), Ok(1));
assert_eq!(graph.degree(&2), Ok(1));
```

### Weighted Directed Graph

```rust
use multigraphrs::{MultiGraph, WeightedDirected};

let mut graph = MultiGraph::<&str, f64, WeightedDirected>::new();

graph.add_node("NYC").unwrap();
graph.add_node("LON").unwrap();

let edge = graph.add_edge("NYC", "LON", 5585.0).unwrap();
assert_eq!(*edge.get_target(), "LON");
assert_eq!(*edge.get_weight(), 5585.0);
```

### Weighted Undirected Graph

```rust
use multigraphrs::{MultiGraph, Weighted};

let mut graph = MultiGraph::<u32, f64, Weighted>::new();

graph.add_node(1).unwrap();
graph.add_node(2).unwrap();

// Weighted undirected: bidirectional edges with the same weight
let edge = graph.add_edge(1, 2, 42.5).unwrap();
assert_eq!(*edge.get_target(), 2);
assert_eq!(*edge.get_weight(), 42.5);
```

## Strategies

| Strategy | Directed | Weighted | `add_edge` signature | Internal edges |
| :--- | :---: | :---: | :--- | :---: |
| `Directed` | ✅ | ❌ (default `1u32`) | `(source, target)` | 1 |
| `Undirected` | ❌ | ❌ (default `1u32`) | `(source, target)` | 2 |
| `WeightedDirected` | ✅ | ✅ | `(source, target, weight)` | 1 |
| `Weighted` | ❌ | ✅ | `(source, target, weight)` | 2 |

## API Overview

### `MultiGraph<K, W, S>`

| Method | Returns | Description |
| :--- | :--- | :--- |
| `new()` | `MultiGraph<K, W, S>` | Create an empty graph |
| `add_node(key)` | `Result<K, GraphErrors>` | Insert a node |
| `remove_node(&key)` | `Result<K, GraphErrors>` | Remove a node and all its edges |
| `add_edge(...)` | `Result<EdgeView<K, W>, GraphErrors>` | Insert an edge (signature varies by strategy) |
| `remove_edge(...)` | `Result<EdgeView<K, W>, GraphErrors>` | Remove an edge by exact match |
| `degree(&key)` | `Result<usize, GraphErrors>` | Number of edges incident to a node |
| `contains_node(&key)` | `bool` | Check if a node exists |
| `contains_edge(&src, &tgt)` | `bool` | Check if an edge exists |
| `node_count()` | `usize` | Total number of nodes |
| `edge_count()` | `usize` | Total number of edges |
| `get_neighbours(&key)` | `Result<Vec<EdgeView<K, W>>, GraphErrors>` | Get all edges from a node |
| `iter()` | `NodeIter<K, W, S>` | Iterate over `(&K, Vec<EdgeView<K, W>>)` |

### `EdgeView<K, W>`

| Method | Returns | Description |
| :--- | :--- | :--- |
| `get_target()` | `&K` | The target node key |
| `get_weight()` | `&W` | The edge weight |

### `GraphErrors`

| Variant | Trigger |
| :--- | :--- |
| `NodeNotFound` | Operating on a non-existent node |
| `NodeAlreadyExists` | Adding a duplicate node |
| `EdgeDoesntExists` | Removing an edge that doesn't exist |

## Architecture

```
MultiGraph<K, W, S>
├── hashed_nodes: HashMap<K, usize>        // user key → internal index
├── reversed_hashed_nodes: Vec<Option<K>>   // internal index → user key (None if removed)
├── adjacency_list: AdjacencyList<W>       // Vec<Vec<Edge<W>>>
└── _strategy: PhantomData<S>              // zero-cost strategy marker

DirectionStrategy<W>  (trait)
├── Directed          ── add_edge → 1 edge   │ remove_edge → match by target
├── Undirected        ── add_edge → 2 edges  │ remove_edge → match by target
├── WeightedDirected  ── add_edge → 1 edge   │ remove_edge → match by target + weight
└── Weighted          ── add_edge → 2 edges  │ remove_edge → match by target + weight
```

## Performance

Benchmarks were conducted on a 10,000,000 node graph with 10,000,000 edges in a simple chain structure.

| Metric | `u32` Nodes | `String` Nodes |
| :--- | :--- | :--- |
| **Add 10M Nodes** | 1.25s | 4.66s |
| **Add 10M Edges** | 3.23s | 4.02s |
| **Iteration** | 202ms | 406ms |
| **Peak Memory** | **1.31 GB** | **2.31 GB** |

> *Benchmarks run in `--release` mode. String overhead reflects 10,000,000 heap-allocated keys.*
> You can run this yourself using: `cargo run --example stress_test --release`

## Roadmap

- [x] Core query methods: `contains_node`, `contains_edge`, `node_count`, `edge_count`, `get_neighbours`
- [x] `remove_node` operation
- [x] Iterator support (`graph.iter()` yields `(&K, Vec<EdgeView<K, W>>)`)
- [ ] `IntoIterator` implementation (for `&graph` in for-loops)
- [ ] Eliminate the `W` generic for unweighted strategies (associated type)
- [ ] Standard trait implementations (`Default`, `Debug`, `Display`)
- [ ] `Display` for `GraphErrors` + `impl std::error::Error`
- [ ] Builder / `from_edges` constructor

## License

This project is licensed under the [MIT License](LICENSE).

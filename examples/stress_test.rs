use multigraphrs::{MultiGraph, Directed};
use std::time::Instant;

fn main() {
    let mut graph = MultiGraph::<String, u32, Directed>::new();
    let node_count = 10_000_000;

    println!("Adding {} String nodes...", node_count);
    let start = Instant::now();
    
    for i in 0..node_count {
        graph.add_node(i.to_string()).unwrap();
    }
    println!("Time to add nodes: {:?}", start.elapsed());

    println!("Adding 10,000,000 edges...");
    let start_edges = Instant::now();
    for i in 0..(node_count - 1) {
        graph.add_edge(i.to_string(), (i + 1).to_string()).unwrap();
    }
    println!("Time to add edges: {:?}", start_edges.elapsed());

    println!("Iterating over graph...");
    let start_iter = Instant::now();
    let mut count = 0;
    let mut edge_count = 0;
    for (_node, edges) in graph.iter() {
        count += 1;
        edge_count += edges.len();
    }
    println!("Time to iterate: {:?}", start_iter.elapsed());
    println!("Total nodes visited: {}, Total edges seen: {}", count, edge_count);
    println!("Total execution time: {:?}", start.elapsed());
}

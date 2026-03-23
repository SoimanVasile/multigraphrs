use multigraphrs::{MultiGraph, Directed};
use std::time::Instant;

fn main() {
    let mut graph = MultiGraph::<u32, u32, Directed>::new();
    let node_count = 10_000_000;

    println!("Adding {} nodes...", node_count);
    let start = Instant::now();
    
    for i in 0..node_count {
        graph.add_node(i).unwrap();
    }
    
    println!("Time to add nodes: {:?}", start.elapsed());
}

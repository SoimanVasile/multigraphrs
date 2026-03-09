use multigraphrs::MultiGraph;
use multigraphrs::Weighted;

#[test]
pub fn check_construction(){
    let mut new_graph = MultiGraph::<u32, u32, Weighted>::new();
    new_graph.add_edge(1, 2, 1);
}

use multigraphrs::{RamMultiGraph, Directed, Weighted};

#[test]
fn returns_user_key_directed() {
    let mut g = RamMultiGraph::<String, u32, Directed>::new();
    g.add_node("Paris".into()).unwrap();
    g.add_node("Berlin".into()).unwrap();

    let edge = g.add_edge("Paris".into(), "Berlin".into()).unwrap();
    assert_eq!(*edge.get_target(), "Berlin".to_string().to_string().to_string());
    assert_eq!(*edge.get_weight(), 1);
}

#[test]
fn returns_user_key_weighted() {
    let mut g = RamMultiGraph::<String, f64, Weighted>::new();
    g.add_node("Tokyo".to_string()).unwrap();
    g.add_node("Seoul".to_string()).unwrap();

    let edge = g.add_edge("Tokyo".to_string(), "Seoul".to_string(), 1159.0).unwrap();
    assert_eq!(*edge.get_target(), "Seoul".to_string().to_string().to_string());
    assert_eq!(*edge.get_weight(), 1159.0);
}

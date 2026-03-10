#[derive(Debug, PartialEq, Eq)]
pub enum GraphErrors{
    NodeNotFound,
    EdgeAlreadyExists,
    NodeAlreadyExists,
}

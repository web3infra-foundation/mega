#[derive(Clone, Debug)]
pub enum GraphError {
    GraphLoopDetected,
    GraphNotActive,
    ExecutionFailed {
        node_name: String,
        node_id: usize,
        error: String,
    },
    PanicOccurred {
        node_name: String,
        node_id: usize,
    },
    MultipleErrors(Vec<GraphError>),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

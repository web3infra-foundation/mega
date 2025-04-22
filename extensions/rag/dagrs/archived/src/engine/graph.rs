/*!
Task Graph

# Graph stores dependency relations.

[`Graph`] represents a series of tasks with dependencies, and stored in an adjacency
list. It must be a directed acyclic graph, that is, the dependencies of the task
cannot form a loop, otherwise the engine will not be able to execute the task successfully.
It has some useful methods for building graphs, such as: adding edges, nodes, etc.
And the most important of which is the `topo_sort` function, which uses topological
sorting to generate the execution sequence of tasks.

# An example of a directed acyclic graph

task1 -→ task3 ---→ task6 ----
 |   ↗   ↓          ↓         ↘
 |  /   task5 ---→ task7 ---→ task9
 ↓ /      ↑          ↓         ↗
task2 -→ task4 ---→ task8 ----

The task execution sequence can be as follows:
task1->task2->task3->task4->task5->task6->task7->task8->task9

*/

use bimap::BiMap;

#[derive(Debug, Clone)]
/// Graph Struct
pub(crate) struct Graph {
    size: usize,
    /// Record node id and it's index <id,index>
    nodes: BiMap<usize, usize>,
    /// Adjacency list of graph (stored as a vector of vector of indices)
    adj: Vec<Vec<usize>>,
    /// Node's in_degree, used for topological sort
    in_degree: Vec<usize>,
}

impl Graph {
    /// Allocate an empty graph
    pub(crate) fn new() -> Graph {
        Graph {
            size: 0,
            nodes: BiMap::new(),
            adj: Vec::new(),
            in_degree: Vec::new(),
        }
    }

    /// Set graph size, size is the number of tasks
    pub(crate) fn set_graph_size(&mut self, size: usize) {
        self.size = size;
        self.adj.resize(size, Vec::new());
        self.in_degree.resize(size, 0);
        self.nodes.reserve(size);
    }

    /// Add a node into the graph
    /// This operation will create a mapping between ID and its index.
    /// **Note:** `id` won't get repeated in dagrs,
    /// since yaml parser will overwrite its info if a task's ID repeats.
    pub(crate) fn add_node(&mut self, id: usize) {
        let index = self.nodes.len();
        self.nodes.insert(id, index);
    }

    /// Add an edge into the graph.
    /// Above operation adds a arrow from node 0 to node 1,
    /// which means task 0 shall be executed before task 1.
    pub(crate) fn add_edge(&mut self, v: usize, w: usize) {
        self.adj[v].push(w);
        self.in_degree[w] += 1;
    }

    /// Find a task's index by its ID
    pub(crate) fn find_index_by_id(&self, id: &usize) -> Option<usize> {
        self.nodes.get_by_left(id).copied()
    }

    /// Find a task's ID by its index
    pub(crate) fn find_id_by_index(&self, index: usize) -> Option<usize> {
        self.nodes.get_by_right(&index).copied()
    }

    /// Do topo sort in graph, returns a possible execution sequence if DAG.
    /// This operation will judge whether graph is a DAG or not,
    /// returns Some(Possible Sequence) if yes, and None if no.
    ///
    ///
    /// **Note**: this function can only be called after graph's initialization (add nodes and edges, etc.) is done.
    ///
    /// # Principle
    /// Reference: [Topological Sorting](https://www.jianshu.com/p/b59db381561a)
    ///
    /// 1. For a graph g, we record the in-degree of every node.
    ///
    /// 2. Each time we start from a node with zero in-degree, name it N0, and N0 can be executed since it has no dependency.
    ///
    /// 3. And then we decrease the in-degree of N0's children (those tasks depend on N0), this would create some new zero in-degree nodes.
    ///
    /// 4. Just repeat step 2, 3 until no more zero degree nodes can be generated.
    ///    If all tasks have been executed, then it's a DAG, or there must be a loop in the graph.
    pub(crate) fn topo_sort(&self) -> Option<Vec<usize>> {
        let mut queue = self
            .in_degree
            .iter()
            .enumerate()
            .filter_map(|(index, &degree)| if degree == 0 { Some(index) } else { None })
            .collect::<Vec<_>>();

        let mut in_degree = self.in_degree.clone();

        let mut sequence = Vec::with_capacity(self.size);

        while let Some(v) = queue.pop() {
            sequence.push(v);

            for &index in self.adj[v].iter() {
                in_degree[index] -= 1;
                if in_degree[index] == 0 {
                    queue.push(index)
                }
            }
        }

        if sequence.len() < self.size {
            None
        } else {
            Some(sequence)
        }
    }

    /// Get the out degree of a node.
    pub(crate) fn get_node_out_degree(&self, id: &usize) -> usize {
        match self.nodes.get_by_left(id) {
            Some(index) => self.adj[*index].len(),
            None => 0,
        }
    }

    /// Get all the successors of a node (direct or indirect).
    /// This function will return a vector of indices of successors (including itself).
    pub(crate) fn get_node_successors(&self, id: &usize) -> Vec<usize> {
        match self.nodes.get_by_left(id) {
            Some(index) => {
                // initialize a vector to store successors with max possible size
                let mut successors = Vec::with_capacity(self.adj[*index].len());

                // create a visited array to avoid visiting a node more than once
                let mut visited = vec![false; self.size];

                // do BFS traversal starting from current node

                // mark the current node as visited and enqueue it
                visited[*index] = true;
                successors.push(*index);

                // the index of the queue
                let mut i_queue = 0;

                // while the queue is not empty
                while i_queue < successors.len() {
                    let v = successors[i_queue];

                    for &index in self.adj[v].iter() {
                        // if not visited, mark it as visited and collect it
                        if !visited[index] {
                            visited[index] = true;
                            successors.push(index);
                        }
                    }
                    i_queue += 1;
                }
                successors
            }
            // If node not found, return empty vector
            None => Vec::new(),
        }
    }
}

impl Default for Graph {
    fn default() -> Self {
        Graph {
            size: 0,
            nodes: BiMap::new(),
            adj: Vec::new(),
            in_degree: Vec::new(),
        }
    }
}

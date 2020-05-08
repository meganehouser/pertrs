use anyhow::{bail, Result};
use petgraph::{Direction, {algo::all_simple_paths, graph::{EdgeIndex, Graph, NodeIndex}, dot::Dot}};
use serde::Deserialize;
use std::fmt;
use std::collections::{HashMap, HashSet};
use itertools::Itertools;


type PertGraph = Graph<NodeWeight, Task>;

fn start_node(graph: &PertGraph) -> Result<NodeIndex<u32>> {
    let start_node: Vec<NodeIndex<u32>> = graph.node_indices().filter(|n| {
        graph.neighbors_directed(*n, Direction::Incoming).next().is_none()
    }).collect();
    match &start_node[..] {
        [] => bail!("Start node is not exist"),
        [node_index] => Ok(*node_index),
        _ => bail!("Start node is duplicated"),
    }
}

fn end_node(graph: &PertGraph) -> Result<NodeIndex<u32>> {
    let end_node: Vec<NodeIndex<u32>> = graph.node_indices().filter(|n| {
        graph.neighbors_directed(*n, Direction::Outgoing).next().is_none()
    }).collect();
    match &end_node[..] {
        [] => bail!("End node is not exist"),
        [node_index] => Ok(*node_index),
        _ => bail!("End node is duplicated"),
    }
}

fn compute_fastest_begin(graph: &PertGraph, from: &NodeIndex<u32>, to: &NodeIndex<u32>) -> u32 {
    all_simple_paths(graph, from.to_owned(), to.to_owned(), 0, None)
        .map(|path: Vec<NodeIndex<u32>>| {
            path.iter()
                .tuple_windows()
                .fold(0u32, |begin, (n1, n2)| {
                    let edge = graph.find_edge(n1.to_owned(), n2.to_owned()).unwrap();
                    let task = graph.edge_weight(edge).unwrap();
                    begin + task.duration
                })
        })
        .max()
        .unwrap_or(0)
}

fn compute_latest_finish(graph: &PertGraph, from: &NodeIndex<u32>, to: &NodeIndex<u32>) -> u32 {
    let total_time = graph.node_weight(to.to_owned()).unwrap().fastest_begin;
    all_simple_paths(graph, from.to_owned(), to.to_owned(), 0, None)
        .map(|path: Vec<NodeIndex<u32>>| {
            path.iter()
                .tuple_windows()
                .fold(total_time, |finish, (n1, n2)| {
                    let edge = graph.find_edge(n1.to_owned(), n2.to_owned()).unwrap();
                    let task = graph.edge_weight(edge).unwrap();
                    finish - task.duration
                })
        })
        .min()
        .unwrap_or(total_time)
}

struct Floats {
    total_float: u32,
    free_float: u32,
}
fn compute_floats(graph: &PertGraph, edge_index: &EdgeIndex<u32>) -> Floats {
    let (begin_index, finish_index) = graph.edge_endpoints(*edge_index).unwrap();
    let begin_node_weight = graph.node_weight(begin_index).unwrap();
    let finish_node_weight = graph.node_weight(finish_index).unwrap();
    let duration = graph.edge_weight(*edge_index).unwrap().duration;
    let total_float = finish_node_weight.latest_finish - (begin_node_weight.fastest_begin + duration);
    let free_float = finish_node_weight.fastest_begin - (begin_node_weight.fastest_begin + duration);
    Floats {
        total_float,
        free_float,
    }
}

struct Pert(PertGraph);

impl Pert {
    fn new(mut graph: PertGraph) -> Result<Pert> {
        let start_node_index = start_node(&graph)?;
        for node_index in graph.node_indices() {
            let fastest_begin = compute_fastest_begin(&graph, &start_node_index, &node_index);
            graph.node_weight_mut(node_index).unwrap().fastest_begin = fastest_begin;
        }

        let end_node_index = end_node(&graph)?;
        for node_index in graph.node_indices() {
            let latest_finish = compute_latest_finish(&graph, &node_index, &end_node_index);
            graph.node_weight_mut(node_index).unwrap().latest_finish = latest_finish;
        }

        for edge_index in graph.edge_indices() {
            let floats = compute_floats(&graph, &edge_index);
            let edge_task = graph.edge_weight_mut(edge_index).unwrap();
            edge_task.total_float = floats.total_float;
            edge_task.free_float = floats.free_float;
        }

        Ok(Pert(graph))
    }
}

#[derive(Deserialize)]
struct Row {
    from: u32,
    to: u32,
    weight: u32,
    name: String,
}

struct DataLoader {
    rows: Vec<Row>,
}

impl DataLoader {
    fn from_bytes(data: &[u8]) -> Result<DataLoader> {
        let rows: Result<Vec<Row>> = csv::ReaderBuilder::new()
            .trim(csv::Trim::Fields)
            .has_headers(false)
            .from_reader(data)
            .records()
            .map(|result| {
                Ok(result?.deserialize(None)?)
            })
            .collect();
        let rows = rows?;
        Ok(DataLoader { rows })
    }

    fn to_graph(&self) -> Result<Pert> {
        let mut graph = PertGraph::new();
        let node_index_map: HashMap::<u32, NodeIndex<u32>> = self.rows.iter()
            .flat_map(|r| vec![r.from, r.to].into_iter())
            .collect::<HashSet<u32>>()
            .iter()
            .map(|u| {
                let idx = graph.add_node(NodeWeight::new(u));
                (*u, idx)
            }).collect();

        for row in self.rows.iter() {
            let from_node = node_index_map.get(&row.from).unwrap();
            let to_node = node_index_map.get(&row.to).unwrap();
            graph.add_edge(
                *from_node,
                *to_node,
                Task::new(&row.name, row.weight),
            );
        }
        Ok(Pert::new(graph)?)
    }
}

#[derive(Debug)]
struct Task {
    name: String,
    duration: u32,
    total_float: u32,
    free_float: u32,
}

impl Task {
    fn new(name: &str, duration: u32) -> Task {
        Task {
            name: String::from(name),
            total_float: 0,
            free_float: 0,
            duration,
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})\nT: {} / F: {}", &self.name, self.duration, self.total_float, self.free_float)
    }
}

#[derive(Debug)]
struct NodeWeight {
    label: u32,
    fastest_begin: u32,
    latest_finish: u32,
}

impl NodeWeight {
    fn new(label: &u32) -> NodeWeight {
        NodeWeight {
            label: *label,
            fastest_begin: 0,
            latest_finish: 0,
        }
    }
}

impl fmt::Display for NodeWeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}\n{}..{}", self.label, self.fastest_begin, self.latest_finish)
    }
}
fn main() -> Result<()> {
    let data = "\
1, 2, 1, t1-2
2, 3, 3, t2-3
1, 3, 5, t1-3
1, 4, 10, t1-4
3, 4, 2, t3-4
";
    let data_loader = DataLoader::from_bytes(data.as_bytes())?;
    let pert = data_loader.to_graph()?;

    println!("{}", Dot::new(&pert.0));
    Ok(())
}

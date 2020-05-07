use anyhow::Result;
use petgraph::{Direction, {algo::all_simple_paths, graph::{Graph, NodeIndex}, dot::Dot}};
use serde::Deserialize;
use std::fmt;
use std::collections::{HashMap, HashSet};
use itertools::Itertools;

#[derive(Debug)]
struct Task {
    name: String,
    weight: u32,
    //total_float: u32,
    // free_float: u32,
}

impl Task {
    fn new(name: &str, weight: u32) -> Task {
        Task {
            name: String::from(name),
            weight,
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Edge {}({})", &self.name, self.weight)
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
            label: label.to_owned(),
            fastest_begin: 0,
            latest_finish: 0,
        }
    }
}

impl fmt::Display for NodeWeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Node({}) {} | {}", self.label, self.latest_finish, self.fastest_begin)
    }
}

#[derive(Deserialize)]
struct Row {
    from: u32,
    to: u32,
    weight: u32,
    name: String,
}

fn _get_all_paths(graph: &Graph<NodeWeight, Task>, start: &NodeIndex<u32>, index: &NodeIndex<u32>, paths: &mut Vec<Vec<NodeIndex<u32>>>) {
    let neighbors: Vec<NodeIndex<u32>> = graph
        .neighbors_directed(index.to_owned(), Direction::Outgoing)
        .collect();

    for n in neighbors.iter() {
        for path in all_simple_paths(graph, start.to_owned(), n.to_owned(), 0, None) {
            paths.push(path);
        }
    }

    for n in neighbors.iter() {
        _get_all_paths(graph, start, &n, paths);
    }
}

fn get_all_paths(graph: &Graph<NodeWeight, Task>, index: &NodeIndex<u32>) -> Vec<Vec<NodeIndex<u32>>> {
    let mut all_paths = Vec::<Vec<NodeIndex<u32>>>::new();
    _get_all_paths(graph, index, index, &mut all_paths);
    all_paths
}

fn compute_fastest_begin(graph: &Graph<NodeWeight, Task>, from: &NodeIndex<u32>, to: &NodeIndex<u32>) -> u32 {
    all_simple_paths(graph, from.to_owned(), to.to_owned(), 0, None)
        .map(|mut path: Vec<NodeIndex<u32>>| {
            path.iter()
                .tuple_windows()
                .fold(0u32, |begin, (n1, n2)| {
                    let edge = graph.find_edge(n1.to_owned(), n2.to_owned()).unwrap();
                    let task = graph.edge_weight(edge).unwrap();
                    begin + task.weight
                })
        })
        .max()
        .unwrap_or(0)
}

fn compute_latest_finish(graph: &Graph<NodeWeight, Task>, from: &NodeIndex<u32>, to: &NodeIndex<u32>) -> u32 {
    let total_time = graph.node_weight(to.to_owned()).unwrap().fastest_begin;
    all_simple_paths(graph, from.to_owned(), to.to_owned(), 0, None)
        .map(|mut path: Vec<NodeIndex<u32>>| {
            path.iter()
                .tuple_windows()
                .fold(total_time, |finish, (n1, n2)| {
                    let edge = graph.find_edge(n1.to_owned(), n2.to_owned()).unwrap();
                    let task = graph.edge_weight(edge).unwrap();
                    finish - task.weight
                })
        })
        .min()
        .unwrap_or(total_time)
}

fn main() -> Result<()> {
    let mut graph = Graph::<NodeWeight, _>::new();
    let data = "\
1, 2, 5, task1
1, 3, 5, task2
2, 4, 3, task3
3, 4, 1, task4
4, 5, 2, task5
";
    let rows: Vec<Row> = csv::ReaderBuilder::new()
        .trim(csv::Trim::Fields)
        .has_headers(false)
        .from_reader(data.as_bytes())
        .records()
        .map(|result| {
            let record = result.unwrap();
            record.deserialize(None).unwrap()
        })
        .collect();

    let node_indices: HashMap::<u32, NodeIndex<u32>> = rows.iter()
        .flat_map(|r| vec![r.from, r.to].into_iter())
        .collect::<HashSet<u32>>()
        .iter()
        .map(|u| {
            let idx = graph.add_node(NodeWeight::new(u));
            (u.to_owned(), idx)
        }).collect();

    for row in rows.iter() {
        let from_node = node_indices.get(&row.from).unwrap();
        let to_node = node_indices.get(&row.to).unwrap();
        graph.add_edge(
            from_node.to_owned(),
            to_node.to_owned(),
            Task::new(&row.name, row.weight),
        );
    }

    let start_node = node_indices.get(&1).unwrap();

    let node_indices: Vec<NodeIndex<u32>> = graph.node_indices().collect();
    node_indices.iter().for_each(|node_index| {
        let fastest_begin = compute_fastest_begin(&graph, start_node, node_index);
        let mut weight = graph.node_weight_mut(node_index.to_owned()).unwrap();
        weight.fastest_begin = fastest_begin;
    });

    let end_node = node_indices.iter().find(|n| {
        (&graph).neighbors_directed((*n).to_owned(), Direction::Outgoing).next().is_none()
    }).unwrap();
    dbg!(end_node);
    node_indices.iter().for_each(|node_index| {
        let latest_finish = compute_latest_finish(&graph, node_index, end_node);
        let mut weight = graph.node_weight_mut(node_index.to_owned()).unwrap();
        weight.latest_finish = latest_finish;
    });

    println!("{}", Dot::new(&graph));
    Ok(())
}

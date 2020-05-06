use anyhow::Result;
use petgraph::{Direction, {graph::{Graph, NodeIndex}, dot::Dot}};
use serde::Deserialize;
use std::fmt;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
struct Task {
    name: String,
    weight: u32,
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
        write!(f, "{}({})", &self.name, self.weight)
    }
}

#[derive(Debug)]
struct NodeWeight {
    label: u32,
    value: u32,
}

impl NodeWeight {
    fn new(label: &u32) -> NodeWeight {
        NodeWeight {
            label: label.to_owned(),
            value: 0,
        }
    }
}

impl fmt::Display for NodeWeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Node{}({})", self.label, self.value)
    }
}

#[derive(Deserialize)]
struct Row {
    from: u32,
    to: u32,
    weight: u32,
    name: String,
}

fn calc_node_weight(graph: &mut Graph<NodeWeight, Task>, index: &NodeIndex<u32>) {
    let neighbors: Vec<NodeIndex<u32>> = graph
        .neighbors_directed(index.to_owned(), Direction::Outgoing)
        .collect();

    for n in neighbors.iter() {
        let edge_index = graph.find_edge(index.to_owned(), n.to_owned()).unwrap();
        let edge_task = graph.edge_weight(edge_index).unwrap();
        let edge_weight = edge_task.weight.to_owned();
        let mut node_weight = graph.node_weight_mut(n.to_owned()).unwrap();
        node_weight.value += edge_weight;
    }

    for n in neighbors.iter() {
        calc_node_weight(graph, &n);
    }
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
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::Fields)
        .has_headers(false)
        .from_reader(data.as_bytes());
    let rows: Vec<Row> = rdr.records()
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
        println!("{} -> {}", &row.from, &row.to);
        graph.add_edge(
            from_node.to_owned(),
            to_node.to_owned(),
            Task::new(&row.name, row.weight),
        );
    }

    let start = node_indices.get(&1).unwrap();
    calc_node_weight(&mut graph, start);

    println!("{}", Dot::new(&graph));
    Ok(())
}

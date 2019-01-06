use super::super::indexed_vec::IndexVec;
use super::{DirectedGraph, WithSuccessors, WithNumNodes};

#[cfg(test)]
mod test;

pub fn post_order_from<G: DirectedGraph + WithSuccessors + WithNumNodes>(
    graph: &G,
    start_node: G::Node,
) -> Vec<G::Node> {
    post_order_from_to(graph, start_node, None)
}

pub fn post_order_from_to<G: DirectedGraph + WithSuccessors + WithNumNodes>(
    graph: &G,
    start_node: G::Node,
    end_node: Option<G::Node>,
) -> Vec<G::Node> {
    let mut visited: IndexVec<G::Node, bool> = IndexVec::from_elem_n(false, graph.num_nodes());
    let mut result: Vec<G::Node> = Vec::with_capacity(graph.num_nodes());
    if let Some(end_node) = end_node {
        visited[end_node] = true;
    }
    post_order_walk(graph, start_node, &mut result, &mut visited);
    result
}

fn post_order_walk<G: DirectedGraph + WithSuccessors + WithNumNodes>(
    graph: &G,
    node: G::Node,
    result: &mut Vec<G::Node>,
    visited: &mut IndexVec<G::Node, bool>,
) {
    if visited[node] {
        return;
    }
    visited[node] = true;

    for successor in graph.successors(node) {
        post_order_walk(graph, successor, result, visited);
    }

    result.push(node);
}

pub fn reverse_post_order<G: DirectedGraph + WithSuccessors + WithNumNodes>(
    graph: &G,
    start_node: G::Node,
) -> Vec<G::Node> {
    let mut vec = post_order_from(graph, start_node);
    vec.reverse();
    vec
}

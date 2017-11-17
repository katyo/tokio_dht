use std::net::{Ipv4Addr, SocketAddrV4, SocketAddr};
use std::slice::Iter;
use std::iter::Filter;

use super::{Node, NodeStatus};

/// Maximum number of nodes that should reside in any bucket.
pub const MAX_BUCKET_SIZE: usize = 8;

/// Bucket containing Nodes with identical bit prefixes.
pub struct Bucket<TNodeId> {
    nodes: [Node<TNodeId>; MAX_BUCKET_SIZE],
}

pub type NodeFilter<'a, TNodeId> = Filter<Iter<'a, Node<TNodeId>>, fn(&&Node<TNodeId>) -> bool>;

impl<TNodeId: Default + Copy + Eq> Bucket<TNodeId> {
    /// Create a new Bucket with all Nodes default initialized.
    pub fn new() -> Self {
        let id = TNodeId::default();
        
        let ip = Ipv4Addr::new(0, 0, 0, 0);
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, 0));
        let node = Node::new(id, addr, NodeStatus::Bad);
        
        Bucket {nodes: [node; MAX_BUCKET_SIZE]}
    }

    /// Iterator over all good nodes in the bucket.
    pub fn good_nodes(&self) -> NodeFilter<TNodeId> {
        self.nodes.iter().filter(|node| node.status() == NodeStatus::Good)
    }

    /// Iterator over all good or questionable nodes in the bucket.
    pub fn pingable_nodes(&self) -> NodeFilter<TNodeId> {
        self.nodes.iter().filter(|node| node.status() != NodeStatus::Bad)
    }

    /// Iterator over each node within the bucket.
    ///
    /// For buckets newly created, the initial bad nodes are included.
    pub fn iter(&self) -> Iter<Node<TNodeId>> {
        self.nodes.iter()
    }

    /// Indicates if the bucket needs to be refreshed.
    pub fn needs_refresh(&self) -> bool {
        self.nodes.iter().fold(true, |prev, node| prev && node.status() != NodeStatus::Good)
    }

    /// Attempt to add the given Node to the bucket if it is not in a bad state.
    ///
    /// Returns false if the Node could not be placed in the bucket because it is full.
    pub fn add_node(&mut self, new_node: Node<TNodeId>) -> bool {
        let new_node_status = new_node.status();
        if new_node_status == NodeStatus::Bad {
            return true;
        }

        // See if this node is already in the table, in that case replace it if it
        // has a higher or equal status to the current node.
        if let Some(index) = self.nodes.iter().position(|node| *node == new_node) {
            let other_node_status = self.nodes[index].status();

            if new_node_status >= other_node_status {
                self.nodes[index] = new_node;
            }

            return true;
        }

        // See if any lower priority nodes are present in the table, we cant do
        // nodes that have equal status because we have to prefer longer lasting
        // nodes in the case of a good status which helps with stability.
        let replace_index = self.nodes.iter().position(|node| node.status() < new_node_status);
        if let Some(index) = replace_index {
            self.nodes[index] = new_node;

            true
        } else {
            false
        }
    }
}

use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Maximum wait period before a node becomes questionable.
const MAX_LAST_SEEN_SECS: u64 = 15 * 60;

/// Maximum number of requests before a Questionable node becomes Bad.
const MAX_REFRESH_REQUESTS: usize = 2;

/// Status of the node.
/// Ordering of the enumerations is important, variants higher
/// up are considered to be less than those further down.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeStatus {
    Bad,
    Questionable,
    Good,
}

/// Node participating in the dht.
#[derive(Debug, Copy, Clone)]
pub struct Node<TNodeId> {
    id: TNodeId,
    addr: SocketAddr,
    last_request: Option<Instant>,
    last_response: Option<Instant>,
    refresh_requests: usize,
}

impl<TNodeId: Copy> Node<TNodeId> {
    /// Create a new node with a given status.
    pub fn new(id: TNodeId, addr: SocketAddr, status: NodeStatus) -> Self {
        use self::NodeStatus::*;
        Node {
            id, addr,
            last_response: match status {
                Good => Some(Instant::now()),
                Questionable => Some(Instant::now() - Duration::from_secs(MAX_LAST_SEEN_SECS)),
                Bad => None,
            },
            last_request: None,
            refresh_requests: 0,
        }
    }

    /// Record that we sent the node a request.
    pub fn local_request(&mut self) {
        if self.status() != NodeStatus::Good {
            let num_requests = self.refresh_requests + 1;

            self.refresh_requests = num_requests;
        }
    }

    /// Record that the node sent us a request.
    pub fn remote_request(&mut self) {
        self.last_request = Some(Instant::now());
    }

    /// Record that the node sent us a response.
    pub fn remote_response(&mut self) {
        self.last_response = Some(Instant::now());

        self.refresh_requests = 0;
    }

    pub fn id(&self) -> &TNodeId {
        &self.id
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    /// Current status of the node.
    pub fn status(&self) -> NodeStatus {
        let curr_time = Instant::now();

        match self.recently_responded(curr_time) {
            NodeStatus::Good => return NodeStatus::Good,
            NodeStatus::Bad => return NodeStatus::Bad,
            NodeStatus::Questionable => (),
        };

        self.recently_requested(curr_time)
    }

    // TODO: Verify the two scenarios follow the specification as some cases seem questionable (pun intended), i.e., a node
    // responds to us once, and then requests from us but never responds to us for the duration of the session. This means they
    // could stay marked as a good node even though they could ignore our requests and just sending us periodic requests
    // to keep their node marked as good in our routing table...

    /// First scenario where a node is good is if it has responded to one of our requests recently.
    ///
    /// Returns the status of the node where a Questionable status means the node has responded
    /// to us before, but not recently.
    fn recently_responded(&self, curr_time: Instant) -> NodeStatus {
        // Check if node has ever responded to us
        let since_response = match self.last_response {
            Some(response_time) => curr_time - response_time,
            None => return NodeStatus::Bad,
        };
        
        // Check if node has recently responded to us
        let max_last_response = Duration::from_secs(MAX_LAST_SEEN_SECS);
        if since_response < max_last_response {
            NodeStatus::Good
        } else {
            NodeStatus::Questionable
        }
    }

    /// Second scenario where a node has ever responded to one of our requests and is good if it
    /// has sent us a request recently.
    ///
    /// Returns the final status of the node given that the first scenario found the node to be
    /// Questionable.
    fn recently_requested(&self, curr_time: Instant) -> NodeStatus {
        // Check if the node has recently request from us
        if let Some(request_time) = self.last_request {
            let since_request = curr_time - request_time;
            
            if since_request < Duration::from_secs(MAX_LAST_SEEN_SECS) {
                return NodeStatus::Good;
            }
        }
        
        // Check if we have request from node multiple times already without response
        if self.refresh_requests < MAX_REFRESH_REQUESTS {
            NodeStatus::Questionable
        } else {
            NodeStatus::Bad
        }
    }
}

impl<TNodeId: Eq> Eq for Node<TNodeId> {}

impl<TNodeId: PartialEq> PartialEq<Node<TNodeId>> for Node<TNodeId> {
    fn eq(&self, other: &Node<TNodeId>) -> bool {
        self.id == other.id && self.addr == other.addr
    }
}

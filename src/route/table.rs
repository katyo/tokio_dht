use std::slice::Iter;
use super::{Node, NodeStatus, NodeId, Bucket, NodeFilter};
use super::bucket::MAX_BUCKET_SIZE;

pub const MAX_BUCKETS: usize = 20 * 8;

/// Routing table containing a table of routing nodes as well
/// as the id of the local node participating in the dht.
pub struct Table<TNodeId> {
    // Important: Our node id will always fall within the range
    // of the last bucket in the buckets array.
    buckets: Vec<Bucket<TNodeId>>,
    node_id: TNodeId,
}

impl<TNodeId: Copy + Default + Eq + NodeId> Table<TNodeId> {
    /// Create a new RoutingTable with the given node id as our id.
    pub fn new(node_id: TNodeId) -> Self {
        let buckets = vec![Bucket::new()];

        Table {buckets, node_id}
    }

    /// Return the node id of the RoutingTable.
    pub fn node_id(&self) -> &TNodeId {
        &self.node_id
    }

    /// Iterator over the closest good nodes to the given node id.
    ///
    /// The closeness of nodes has a maximum granularity of a bucket. For most use
    /// cases this is fine since we will usually be performing lookups and aggregating
    /// a number of results equal to the size of a bucket.
    pub fn closest_nodes<'a>(&'a self, node_id: TNodeId) -> ClosestNodes<'a, TNodeId> {
        ClosestNodes::new(&self.buckets, self.node_id, node_id)
    }
    
    /// Iterator over all buckets in the routing table.
    pub fn buckets<'a>(&'a self) -> Buckets<'a, TNodeId> {
        Buckets::new(&self.buckets)
    }
    
    /// Find an instance of the target node in the RoutingTable, if it exists.
    pub fn find_node(&self, node: &Node<TNodeId>) -> Option<&Node<TNodeId>> {
        let bucket_index = self.node_id.equal_bits(node.id());

        // Check the sorted bucket
        let opt_bucket_contents = if let Some(c) = self.buckets().skip(bucket_index).next() {
            // Got the sorted bucket
            Some(c)
        } else {
            // Grab the assorted bucket (if it exists)
            self.buckets().find(|c| {
                match c {
                    &BucketContents::Empty => false,
                    &BucketContents::Sorted(_) => false,
                    &BucketContents::Assorted(_) => true,
                }
            })
        };

        // Check for our target node in our results
        match opt_bucket_contents {
            Some(BucketContents::Sorted(b)) => b.pingable_nodes().find(|n| n == &node),
            Some(BucketContents::Assorted(b)) => b.pingable_nodes().find(|n| n == &node),
            _ => None,
        }
    }

    /// Add the node to the RoutingTable if there is space for it.
    pub fn add_node(&mut self, node: Node<TNodeId>) {
        // Doing some checks and calculations here, outside of the recursion
        if node.status() == NodeStatus::Bad {
            return;
        }
        let num_same_bits = self.node_id.equal_bits(node.id());

        // Should not add a node that has the same id as us
        if num_same_bits != MAX_BUCKETS {
            self.bucket_node(node, num_same_bits);
        }
    }

    /// Recursively tries to place the node into some bucket.
    fn bucket_node(&mut self, node: Node<TNodeId>, num_same_bits: usize) {
        let bucket_index = bucket_placement(num_same_bits, self.buckets.len());

        // Try to place in correct bucket
        if !self.buckets[bucket_index].add_node(node.clone()) {
            // Bucket was full, try to split it
            if self.split_bucket(bucket_index) {
                // Bucket split successfully, try to add again
                self.bucket_node(node.clone(), num_same_bits);
            }
        }
    }

    /// Tries to split the bucket at the specified index.
    ///
    /// Returns false if the split cannot be performed.
    fn split_bucket(&mut self, bucket_index: usize) -> bool {
        if !can_split_bucket(self.buckets.len(), bucket_index) {
            return false;
        }

        // Implementation is easier if we just remove the whole bucket, pretty
        // cheap to copy and we can manipulate the new buckets while they are
        // in the RoutingTable already.
        let split_bucket = match self.buckets.pop() {
            Some(bucket) => bucket,
            None => panic!("No buckets present in RoutingTable, implementation error..."),
        };

        // Push two more buckets to distribute nodes between
        self.buckets.push(Bucket::new());
        self.buckets.push(Bucket::new());

        for node in split_bucket.iter() {
            self.add_node(node.clone());
        }

        true
    }
}

// Iterator filter for only good nodes.
type GoodNodes<'a, TNodeId> = NodeFilter<'a, TNodeId>;

// So what we are going to do here is iterate over every bucket in a hypothetically filled
// routing table (buckets slice). If the bucket we are interested in has not been created
// yet (not in the slice), go through the last bucket (assorted nodes) and check if any nodes
// would have been placed in that bucket. If we find one, return it and mark it in our assorted
// nodes array.
pub struct ClosestNodes<'a, TNodeId: 'a> {
    buckets: &'a [Bucket<TNodeId>],
    current_iter: Option<GoodNodes<'a, TNodeId>>,
    current_index: usize,
    start_index: usize,
    // Since we could have assorted nodes that are interleaved between our sorted
    // nodes as far as closest nodes are concerned, we need some way to hand the
    // assorted nodes out and keep track of which ones we have handed out.
    // (Bucket Index, Node Reference, Returned Before)
    assorted_nodes: Option<[(usize, &'a Node<TNodeId>, bool); MAX_BUCKET_SIZE]>,
}

impl<'a, TNodeId: 'a + Copy + Default + Eq + NodeId> ClosestNodes<'a, TNodeId> {
    fn new(buckets: &'a [Bucket<TNodeId>], self_node_id: TNodeId, other_node_id: TNodeId) -> ClosestNodes<'a, TNodeId> {
        let start_index = self_node_id.equal_bits(&other_node_id);
        let current_index = start_index;

        let current_iter = bucket_iterator(buckets, start_index);
        let assorted_nodes = precompute_assorted_nodes(buckets, self_node_id);

        ClosestNodes {
            buckets,
            current_iter,
            current_index,
            start_index,
            assorted_nodes,
        }
    }
}

/// Optionally returns the filter iterator for the bucket at the specified index.
fn bucket_iterator<'a, TNodeId: Copy + Default + Eq>(buckets: &'a [Bucket<TNodeId>], index: usize) -> Option<GoodNodes<'a, TNodeId>> {
    if buckets.len() == MAX_BUCKETS {
        buckets
    } else {
        &buckets[..(buckets.len() - 1)]
    }
    .get(index).map(|bucket| good_node_filter(bucket.iter()))
}

/// Converts the given iterator into a filter iterator to return only good nodes.
fn good_node_filter<'a, TNodeId: Copy>(iter: Iter<'a, Node<TNodeId>>) -> GoodNodes<'a, TNodeId> {
    iter.filter(|&&node| node.status() != NodeStatus::Bad)
}

/// Optionally returns the precomputed bucket positions for all assorted nodes.
fn precompute_assorted_nodes<'a, TNodeId: Copy + Default + Eq + NodeId>
    (buckets: &'a [Bucket<TNodeId>], self_node_id: TNodeId)
     -> Option<[(usize, &'a Node<TNodeId>, bool); MAX_BUCKET_SIZE]> {
    if buckets.len() == MAX_BUCKETS {
        return None;
    }
    let assorted_bucket = &buckets[buckets.len() - 1];
    let mut assorted_iter = assorted_bucket.iter().peekable();
    
    // So the bucket is not empty and now we have a reference to initialize our stack allocated array.
    if let Some(&init_reference) = assorted_iter.peek() {
        // Set all tuples to true in case our bucket is not full.
        let mut assorted_nodes = [(0, init_reference, true); MAX_BUCKET_SIZE];

        for (index, node) in assorted_iter.enumerate() {
            let bucket_index = self_node_id.equal_bits(node.id());

            assorted_nodes[index] = (bucket_index, node, false);
        }

        Some(assorted_nodes)
    } else {
        None
    }
}

impl<'a, TNodeId: Copy + Default + Eq> Iterator for ClosestNodes<'a, TNodeId> {
    type Item = &'a Node<TNodeId>;

    fn next(&mut self) -> Option<&'a Node<TNodeId>> {
        let current_index = self.current_index;

        // Check if we have any nodes left in the current iterator
        if let Some(ref mut iter) = self.current_iter {
            match iter.next() {
                Some(node) => return Some(node),
                None => (),
            };
        }

        // Check if we have any nodes to give in the assorted bucket
        if let Some(ref mut nodes) = self.assorted_nodes {
            let mut nodes_iter = nodes.iter_mut().filter(|&&mut (_, node, ..)| node.status() != NodeStatus::Bad);

            match nodes_iter.find(|&&mut (index, _, free)| index == current_index && !free) {
                Some(&mut (_, node, ref mut free)) => {
                    *free = true;

                    return Some(node);
                }
                None => (),
            };
        }

        // Check if we can move to a new bucket
        match next_bucket_index(MAX_BUCKETS, self.start_index, self.current_index) {
            Some(new_index) => {
                self.current_index = new_index;
                self.current_iter = bucket_iterator(self.buckets, self.current_index);

                // Recurse back into this function to check the previous code paths again
                self.next()
            }
            None => None,
        }
    }
}

/// Computes the next bucket index that should be visited given the number of buckets, the starting index
/// and the current index.
///
/// Returns None if all of the buckets have been visited.
fn next_bucket_index(num_buckets: usize, start_index: usize, curr_index: usize) -> Option<usize> {
    // Since we prefer going right first, that means if we are on the right side then we want to go
    // to the same offset on the left, however, if we are on the left we want to go 1 past the offset
    // to the right. All assuming we can actually do this without going out of bounds.
    if curr_index == start_index {
        let right_index = start_index.checked_add(1);
        let left_index = start_index.checked_sub(1);

        if index_is_in_bounds(num_buckets, right_index) {
            Some(right_index.unwrap())
        } else if index_is_in_bounds(num_buckets, left_index) {
            Some(left_index.unwrap())
        } else {
            None
        }
    } else if curr_index > start_index {
        let offset = curr_index - start_index;

        let left_index = start_index.checked_sub(offset);
        let right_index = curr_index.checked_add(1);

        if index_is_in_bounds(num_buckets, left_index) {
            Some(left_index.unwrap())
        } else if index_is_in_bounds(num_buckets, right_index) {
            Some(right_index.unwrap())
        } else {
            None
        }
    } else {
        let offset = (start_index - curr_index) + 1;

        let right_index = start_index.checked_add(offset);
        let left_index = curr_index.checked_sub(1);

        if index_is_in_bounds(num_buckets, right_index) {
            Some(right_index.unwrap())
        } else if index_is_in_bounds(num_buckets, left_index) {
            Some(left_index.unwrap())
        } else {
            None
        }
    }
}

/// Returns true if the overflow checked index is in bounds of the given length.
fn index_is_in_bounds(length: usize, checked_index: Option<usize>) -> bool {
    match checked_index {
        Some(index) => index < length,
        None => false,
    }
}

/// Iterator over buckets where the item returned is an enum
/// specifying the current state of the bucket returned.
#[derive(Copy, Clone)]
pub struct Buckets<'a, TNodeId: 'a> {
    buckets: &'a [Bucket<TNodeId>],
    index: usize,
}

impl<'a, TNodeId> Buckets<'a, TNodeId> {
    fn new(buckets: &'a [Bucket<TNodeId>]) -> Buckets<'a, TNodeId> {
        Buckets {
            buckets: buckets,
            index: 0,
        }
    }
}

impl<'a, TNodeId> Iterator for Buckets<'a, TNodeId> {
    type Item = BucketContents<'a, TNodeId>;

    fn next(&mut self) -> Option<BucketContents<'a, TNodeId>> {
        if self.index > MAX_BUCKETS {
            return None;
        } else if self.index == MAX_BUCKETS {
            // If not all sorted buckets were present, return the assorted bucket
            // after the iteration of the last bucket occurs, which is here!
            self.index += 1;

            return if self.buckets.len() == MAX_BUCKETS || self.buckets.is_empty() {
                None
            } else {
                Some(BucketContents::Assorted(&self.buckets[self.buckets.len() - 1]))
            };
        }

        if self.index + 1 < self.buckets.len() || self.buckets.len() == MAX_BUCKETS {
            self.index += 1;

            Some(BucketContents::Sorted(&self.buckets[self.index - 1]))
        } else {
            self.index += 1;

            Some(BucketContents::Empty)
        }
    }
}

#[derive(Copy, Clone)]
pub enum BucketContents<'a, TNodeId: 'a> {
    /// Empty bucket is a placeholder for a bucket that has not yet been created.
    Empty,
    /// Sorted bucket is where nodes with the same leading bits reside.
    Sorted(&'a Bucket<TNodeId>),
    /// Assorted bucket is where nodes with differing bits reside.
    ///
    /// These nodes are dynamically placed in their sorted bucket when is is created.
    Assorted(&'a Bucket<TNodeId>),
}

/*
impl<'a, TNodeId> BucketContents<'a, TNodeId> {
    pub fn is_empty(&self) -> bool {
        match self {
            &BucketContents::Empty => true,
            _ => false,
        }
    }

    pub fn is_sorted(&self) -> bool {
        match self {
            &BucketContents::Sorted(_) => true,
            _ => false,
        }
    }

    pub fn is_assorted(&self) -> bool {
        match self {
            &BucketContents::Assorted(_) => true,
            _ => false,
        }
    }
}
*/

/// Take the number of leading bits that are the same between our node and the remote
/// node and calculate a bucket index for that node id.
fn bucket_placement(num_same_bits: usize, num_buckets: usize) -> usize {
    // The index that the node should be placed in *eventually*, meaning
    // when we create enough buckets for that bucket to appear.
    let ideal_index = num_same_bits;

    if ideal_index >= num_buckets {
        num_buckets - 1
    } else {
        ideal_index
    }
}

/// Returns true if the bucket can be split.
fn can_split_bucket(num_buckets: usize, bucket_index: usize) -> bool {
    bucket_index == num_buckets - 1 && bucket_index != MAX_BUCKETS - 1
}

/*
pub trait Id<H> {
    /// Number of leading bits that are identical between two hashes
    fn equal_bits(&self, hash: &H) -> usize;

    //
    //fn nearest_of(&self, hashes: &[&Hash]) -> usize;
}
*/

pub trait NodeId {
    /// Number of leading bits that are identical between two hashes
    fn equal_bits(&self, other: &Self) -> usize;
}

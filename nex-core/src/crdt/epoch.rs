
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(pub u64);

impl Epoch {
    // Only derive internally
    pub(crate) fn derive(parent_max: u64, has_resurrect: bool) -> Self {
        Epoch(parent_max + if has_resurrect { 1 } else { 0 })
    }
}

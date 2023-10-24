use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serializable_parts", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serializable_parts_rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct Point<T: Debug> {
    pub(crate) x: T,
    pub(crate) y: T,
}

impl<T: Debug> Display for Point<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.x, self.y)
    }
}

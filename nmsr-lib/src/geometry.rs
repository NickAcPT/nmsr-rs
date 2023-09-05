#[cfg(feature = "serializable_parts")]
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serializable_parts", derive(Serialize, Deserialize))]
pub struct Point<T: Debug> {
    pub(crate) x: T,
    pub(crate) y: T,
}

impl<T: Debug> Display for Point<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.x, self.y)
    }
}

#[cfg(feature = "serializable_parts")] use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serializable_parts", derive(Serialize, Deserialize))]
pub struct Point<T: Debug> {
    pub(crate) x: T,
    pub(crate) y: T,
}

#[derive(Debug, Clone)]
pub struct Size<T: Debug> {
    pub(crate) width: T,
    pub(crate) height: T,
}

#[derive(Debug, Clone)]
pub struct Rectangle<T: Debug> {
    pub(crate) position: Point<T>,
    pub(crate) size: Size<T>,
}

impl<T: Debug> Display for Point<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.x, self.y)
    }
}

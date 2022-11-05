pub mod errors;
pub mod parts;
pub mod rendering;
pub mod uv;

#[cfg(feature = "rayon")]
macro_rules! par_iterator_if_enabled {
    ($value: expr) => {
        $value.par_iter()
    };
}

#[cfg(not(feature = "rayon"))]
macro_rules! par_iterator_if_enabled {
    ($value: expr) => {
        $value.iter()
    };
}

pub(crate) use par_iterator_if_enabled;

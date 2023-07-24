pub(crate) mod errors;
#[cfg(feature = "tracing")]
pub(crate) mod tracing_headers;
#[cfg(feature = "tracing")]
pub(crate) mod tracing_span;

pub(crate) type Result<T> = std::result::Result<T, errors::NMSRaaSError>;

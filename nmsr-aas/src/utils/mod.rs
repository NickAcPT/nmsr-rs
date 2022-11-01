pub(crate) mod errors;

pub(crate) type Result<T> = std::result::Result<T, errors::NMSRaaSError>;

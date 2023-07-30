use thiserror::Error;

#[derive(Debug, Error)]
pub enum NMSRRenderingError {
    #[error("Unable to find a suitable backend. Either pass in a backend or set the WGPU_BACKEND environment variable")]
    NoBackendFound,
    #[error("Unable to find a suitable adapter")]
    NoAdapterFound,
    #[error("Unable to create a device: {0}")]
    WgpuRequestDeviceError(#[from] wgpu::RequestDeviceError),
    #[error("Surface is not supported by the adapter")]
    SurfaceNotSupported,
    #[error("Unable to request adapter: {0}")]
    WgpuAdapterRequestError
}

pub(crate) type Result<T> = std::result::Result<T, NMSRRenderingError>;
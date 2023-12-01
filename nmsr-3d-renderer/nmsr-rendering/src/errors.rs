use nmsr_player_parts::types::PlayerPartTextureType;
use thiserror::Error;
use tokio::sync::oneshot::error::RecvError;

#[derive(Debug, Error)]
pub enum NMSRRenderingError {
    #[error("Unable to find a suitable backend. Either pass in a backend or set the WGPU_BACKEND environment variable")]
    NoBackendFound,
    #[error("Unable to find a suitable adapter")]
    NoAdapterFound,
    #[cfg(feature = "pipeline")]
    #[error("Unable to create a device: {0}")]
    WgpuRequestDeviceError(#[from] wgpu::RequestDeviceError),
    #[error("Surface is not supported by the adapter")]
    SurfaceNotSupported,
    #[error("Unable to request adapter")]
    WgpuAdapterRequestError,
    #[error("SceneContext textures not initialized")]
    SceneContextTexturesNotInitialized,
    #[error("SceneContext Texture not set: {0}")]
    SceneContextTextureNotSet(PlayerPartTextureType),
    #[cfg(feature = "pipeline")]
    #[error("Buffer Async error: {0}")]
    BufferAsyncError(#[from] wgpu::BufferAsyncError),
    #[error("RecvError: {0}")]
    RecvError(#[from] RecvError),
    #[error("Unable to convert image from raw bytes: {0}")]
    ImageFromRawError(image::ImageError),
    #[cfg(feature = "pipeline")]
    #[error("Pool error: {0}")]
    PoolError(#[from] deadpool::managed::PoolError<Box<Self>>),
    #[cfg(feature = "pipeline")]
    #[error("Pool Build error: {0}")]
    PoolBuildError(#[from] deadpool::managed::BuildError),
}

pub(crate) type Result<T> = std::result::Result<T, NMSRRenderingError>;

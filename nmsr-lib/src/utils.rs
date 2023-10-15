use image::RgbaImage;
use vfs::VfsPath;

use crate::errors::NMSRError;
use crate::errors::Result;

pub(crate) fn open_image_from_vfs(path: &VfsPath) -> Result<RgbaImage> {
    let len = path.metadata()?.len;
    let mut buf = Vec::with_capacity(len as usize);

    let _ = path
        .open_file()?
        .read_to_end(&mut buf)
        .map_err(|e| NMSRError::UnspecifiedIoError(format!("Failed to read file: {}", e)))?;

    let (header, pixels) = qoi::decode_to_vec(buf)
        .map_err(|_| NMSRError::UnspecifiedIoError("Failed to decode image".to_string()))?;

    RgbaImage::from_raw(header.width, header.height, pixels)
        .ok_or_else(|| NMSRError::UnspecifiedIoError("Failed to create image".to_string()))
}

#[cfg(feature = "parallel_iters")]
macro_rules! par_iterator_if_enabled {
    ($value: expr) => {
        $value.par_iter()
    };
}

#[cfg(not(feature = "parallel_iters"))]
macro_rules! par_iterator_if_enabled {
    ($value: expr) => {
        $value.iter()
    };
}

#[cfg(feature = "parallel_iters")]
macro_rules! into_par_iter_if_enabled {
    ($value: expr) => {
        $value.into_par_iter()
    };
}

#[cfg(not(feature = "parallel_iters"))]
macro_rules! into_par_iter_if_enabled {
    ($value: expr) => {
        $value.into_iter()
    };
}

pub(crate) use into_par_iter_if_enabled;
pub(crate) use par_iterator_if_enabled;

use std::io::BufReader;
use image::DynamicImage;
use vfs::VfsPath;
use crate::errors::NMSRError;
use crate::errors::Result;

pub(crate) fn open_image_from_vfs(path: &VfsPath) -> Result<DynamicImage> {
    let reader = BufReader::new(path.open_file()?);
    let image = image::load(reader, image::ImageFormat::Png)
        .map_err(NMSRError::ImageError)?;

    Ok(image)
}

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

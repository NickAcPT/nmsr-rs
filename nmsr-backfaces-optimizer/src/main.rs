use std::{env, fs};

use anyhow::Result;
use image::{GenericImage, GenericImageView, Rgba};
use walkdir::WalkDir;

fn main() -> Result<()> {
    for x in WalkDir::new(env::current_dir()?) {
        let entry = x?;
        let path = entry.path();
        let file_name = path
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("Invalid path {:?}", path.to_owned()))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path {:?}", path.to_owned()))?;

        if !path.is_file() {
            continue;
        }

        if file_name.contains("BackFaces") && !file_name.contains("Layer") {
            fs::remove_file(path)?;
            continue;
        } else if !file_name.contains("Layer BackFaces") {
            continue;
        }

        let normal_path = path.with_file_name(format!(
            "{}.png",
            file_name
                .replace("Layer BackFaces", "")
                .replace(" 0", "0")
                .replace("Hat", "Head")
                .trim()
        ));
        if !normal_path.exists() {
            println!(
                "Missing normal path: {} for {}",
                normal_path.display(),
                path.display()
            );
            continue;
        }

        let mut backfaces_image = image::open(path)?.into_rgba16();
        let normal_image = image::open(normal_path)?;

        unsafe {
            let pixels = normal_image.pixels();

            let empty = Rgba([0u16, 0u16, 0u16, 0u16]);
            for (x, y, pixel) in pixels {
                if pixel.0[3] > 0 {
                    backfaces_image.unsafe_put_pixel(x, y, empty)
                }
            }
        }

        backfaces_image.save(path)?;
    }

    Ok(())
}

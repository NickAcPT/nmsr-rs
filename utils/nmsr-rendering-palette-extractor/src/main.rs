use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Ok, Result};
use heck::ToUpperCamelCase;
use image::ImageFormat;

fn main() -> Result<()> {
    let path: PathBuf = "color_palettes".into();

    let read_dir = path.read_dir().context("Failed to read directory")?;

    let mut entries = Vec::new();

    for dir_entry in read_dir {
        let entry = dir_entry.context("Failed to read directory entry")?;
        let entry = entry.path();

        let entry_bytes = fs::read(&entry).context("Failed to read file")?;

        let palette_img =
            image::load_from_memory_with_format(&entry_bytes, ImageFormat::Png)?.to_rgb8();

        let palette_name = entry
            .file_stem()
            .ok_or(anyhow!("Failed to get file name"))?
            .to_str()
            .ok_or(anyhow!("Failed to convert file name to str"))?
            .to_upper_camel_case();

        let mut pixels: Vec<_> = palette_img
            .pixels()
            .collect();
        
        pixels.sort_by_key(|f| f.0);
        
        let pixels = pixels.into_iter()
            .map(|x| format!("[0x{:02X}, 0x{:02X}, 0x{:02X}]", x[0], x[1], x[2]))
            .collect::<Vec<_>>()
            .join(", ");

        entries.push((palette_name, pixels));
    }

    entries.sort_by_key(|x| x.0.clone());

    for (palette_name, pixels) in entries {
        println!("Self::{} => [{}],", palette_name, pixels);
    }

    Ok(())
}

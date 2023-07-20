use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::{env, fs};

use anyhow::{anyhow, Result};
use image::{GenericImage, ImageBuffer, Rgba};
use rayon::prelude::*;

pub(crate) type Rgba16Image = ImageBuffer<Rgba<u16>, Vec<u16>>;

fn main() -> Result<()> {
    let dir = env::current_dir()?;
    let root = dir.as_path();

    merge_parts_in_directory(root, root)?;

    Ok(())
}

fn merge_parts_in_directory(dir: &Path, root: &Path) -> Result<()> {
    println!("Merging parts in directory: {}", dir.display());

    // Organize by BackFaces, then by the Layers, then all the remaining things
    let groups = vec![
        ("BackFaces", RwLock::new(vec![])),
        ("Layer", RwLock::new(vec![])),
        ("", RwLock::new(vec![])),
    ];

    'outer: for x in dir.read_dir()? {
        let path = x?.path();

        if path.is_file() {
            for (key, group) in &groups {
                let name = path
                    .file_name()
                    .ok_or_else(|| anyhow!("Invalid file name"))?
                    .to_string_lossy();
                if name.contains(key) {
                    if name.contains("environment_background") {
                        let parent = path.parent().ok_or_else(|| anyhow!("Invalid parent"))?;
                        let destination = get_final_dir(parent, root)?.join(
                            path.file_name()
                                .ok_or_else(|| anyhow!("File name is empty"))?,
                        );

                        fs::copy(&path, destination)?;
                    } else {
                        group
                            .write()
                            .expect("Failed to write to group")
                            .push(path.clone());
                    }

                    continue 'outer;
                }
            }
        } else if path.is_dir()
            && !path
                .file_name()
                .expect("Dir needs a filename")
                .to_string_lossy()
                .contains("overlays")
        {
            merge_parts_in_directory(&path, root)?;
        }
    }

    for (key, group) in groups {
        if group.read().unwrap().is_empty() {
            continue;
        }

        println!(
            "Loading images in group {}: {}",
            key,
            group.read().unwrap().len()
        );
        let group = group.read().expect("Failed to read group");
        let group: Vec<_> = (*group)
            .par_iter()
            .map(|i| {
                image::open(i)
                    .expect("Expected image to load")
                    .into_rgba16()
            })
            .collect();

        let first_image = group.first().expect("Group needs at least one image");
        let mut merged_image: Rgba16Image =
            ImageBuffer::new(first_image.width(), first_image.height());

        let mut all_group_pixels: Vec<_> = vec![];

        for image in &group {
            all_group_pixels.par_extend(image.enumerate_pixels().par_bridge());
        }

        all_group_pixels.par_sort_by_key(|(_, _, pixel)| pixel.0[2]); // Sort by depth

        for (x, y, pixel) in all_group_pixels {
            unsafe { merged_image.unsafe_put_pixel(x, y, *pixel) };
        }

        let parent_dir = get_final_dir(dir, root)?;

        let merged_path = parent_dir.join(format!(
            "{}-merged.png",
            if key.is_empty() { "base" } else { key }
        ));

        merged_image.save(&merged_path)?;

        println!("Saved merged image: {}", merged_path.display());
    }

    Ok(())
}

fn get_final_dir(dir: &Path, root: &Path) -> Result<PathBuf> {
    let parent_dir = root
        .parent()
        .unwrap()
        .join("merged-parts")
        .join(dir.strip_prefix(root)?);
    fs::create_dir_all(&parent_dir)?;

    Ok(parent_dir)
}

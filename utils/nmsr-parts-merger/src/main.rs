use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::{env, fs};
use std::ops::Deref;

use anyhow::{anyhow, Result};
use image::{GenericImage, ImageBuffer, Rgba};
use itertools::Itertools;
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
            /*&& !path
                .file_name()
                .expect("Dir needs a filename")
                .to_string_lossy()
                .contains("overlays")*/
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

        println!("Merging images in group {}", key);
        for image in &group {
            all_group_pixels.par_extend(image.enumerate_pixels().par_bridge().map(|p| (image, p)));
        }
        println!("Loaded images in group {}", key);

        println!("Sorting pixels in group {}", key);
        all_group_pixels.par_sort_by_key(|(_, (_, _, pixel))| pixel.0[2]); // Sort by depth
        println!("Sorted pixels in group {}", key);

        println!("Checking for overlapping pixels in group {}", key);
        //Check if we have overlapping pixels, if we have lots of them, we should move them to a separate image

        // For this, we have to go through all the pixels and check the overlapping ones by comparing the depth for each pixel in the x and y axis
        // First, we group the pixels by x and y position
        // Then we take the pixels that have the same x and y position and check if they have the same depth
        // If they have the same depth, we add them to the overlapping pixels list
        // Same depth is defined as the difference between the two depths being less than 100
        let group_by = all_group_pixels
            .iter().group_by(|(_, (x, y, _))| (*x, *y));
        let grouped_pixels = group_by.into_iter();

        let grouped_pixels: Vec<_> = grouped_pixels.collect();

        grouped_pixels.into_iter().max_by(|(_, group), (_, group2)| group.clone().count().cmp(&group2.count())).map(|(key, _)| key).unwrap();

        let mut shown_sample = false;

        let mut overlapping_pixels = vec![];
        for (_, group) in grouped_pixels {
            let mut group = group.collect::<Vec<_>>();
            group.sort_by_key(|(_, (_, _, pixel))| pixel.0[2]);
            let mut group = group.into_iter();
            let mut last_pixel = group.next().unwrap();
            let (_, (_, _, last_pixel_rgba)) = last_pixel;
            for p in group {
                let (_, (x, y, pixel_rgba)) = p;
                if pixel_rgba.0[2] - last_pixel_rgba.0[2] < 100 {
                    overlapping_pixels.push((x, y));

                    if (!shown_sample) {
                        println!("Found overlapping pixels in group {}:", key);
                        println!("Pixel 1: {:?}", last_pixel.1);
                        println!("Pixel 2: {:?}", p.1);
                        shown_sample = true;
                    }
                }
                last_pixel = p;
            }
        }


        for (_, (x, y, &pixel)) in &all_group_pixels {
            if overlapping_pixels.contains(&(x, y)) {
                continue;
            }
            unsafe { merged_image.unsafe_put_pixel(*x, *y, pixel) };
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

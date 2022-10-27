use anyhow::{Context, Result};
use nmsr_lib::parts::manager::PartsManager;
use nmsr_lib::rendering::entry::RenderingEntry;
use std::borrow::Borrow;

fn main() -> Result<()> {
    println!("NickAc's Minecraft Skin Renderer - Initializing...");

    let start = std::time::Instant::now();
    let parts_manager = PartsManager::new("parts").with_context(|| "Failed to load parts")?;
    let end = std::time::Instant::now();
    println!(
        "Loaded {} parts in {:?} ({:?} overlays)",
        parts_manager.all_parts.len() + parts_manager.model_parts.len(),
        end - start,
        parts_manager.borrow().model_overlays.len()
    );

    let entry = RenderingEntry::new(image::open("skin.png")?.into_rgba16(), true);

    entry.render(&parts_manager)?.save("out.png")?;

    Ok(())
}

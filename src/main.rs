use anyhow::{Context, Result};
use nmsr::parts::manager::PartsManager;
use nmsr::rendering::entry::RenderingEntry;

fn main() -> Result<()> {
    println!("NickAc's Minecraft Skin Renderer - Initializing...");

    let start = std::time::Instant::now();
    let parts_manager = PartsManager::new("parts").with_context(|| "Failed to load parts")?;
    let end = std::time::Instant::now();
    println!(
        "Loaded {} parts in {:?}",
        parts_manager.all_parts.len() + parts_manager.model_parts.len(),
        end - start
    );

    let entry = RenderingEntry::new(image::open("skin.png")?.into_rgba8(), false);

    entry.render(&parts_manager).save("out.png")?;

    Ok(())
}

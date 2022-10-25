use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use crate::uv::uv_magic::UvImage;

fn main() {
    println!("NickAc's Minecraft Skin Renderer - Initializing...");

    let start = std::time::Instant::now();
    let parts_manager = PartsManager::new("parts");
    let end = std::time::Instant::now();
    println!(
        "Loaded {} parts in {:?}",
        parts_manager.all_parts.len() + parts_manager.model_parts.len(),
        end - start
    );

    let entry = RenderingEntry::new(image::open("skin.png").unwrap().into_rgba8(), false);

    entry
        .render(&parts_manager)
        .save("out.png")
        .expect("Image should have saved");
}

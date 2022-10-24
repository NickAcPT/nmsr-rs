use crate::parts::parts::PartsManager;
use crate::uv::uv_magic::UvImage;

mod parts;
mod uv;

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

    for (name, _) in parts_manager.all_parts {
        println!("{}", name);
    }
}

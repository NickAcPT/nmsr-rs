use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let types = ["FullBody", "FrontFull"];

    for x in WalkDir::new("../../../parts") {
        let entry = x?;
        let path = entry.path();
        let file_name = path
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

        // Get last word in file name
        let last_word = file_name
            .to_str()
            .and_then(|s| s.split(' ').last())
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

        if !path.is_file() {
            continue;
        }
        if !types.contains(&last_word) {
            continue;
        }
        let new_parent = Path::new("../../../parts").join(last_word);
        // Take all but last word
        let new_file_name = file_name
            .to_str()
            .map(|s| s.split(' '))
            .map(|s| {
                s.take_while(|&s| s != last_word)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;

        // Insert new_parent between parts the file name
        let old_path_without_parent = path
            .strip_prefix("../../../parts")?
            .with_file_name(&new_file_name)
            .with_extension("png");
        let new_path = new_parent.join(old_path_without_parent);

        println!("{:?} -> {:?}", path, new_path);

        // Move file
        std::fs::create_dir_all(
            new_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid path"))?,
        )?;
        std::fs::rename(path, new_path)?;
    }

    Ok(())
}

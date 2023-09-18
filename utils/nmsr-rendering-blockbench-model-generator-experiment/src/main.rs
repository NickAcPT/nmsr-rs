mod blockbench;
mod generator;

use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{anyhow, Context, Ok, Result};
use clap::{Parser, ValueEnum};
use derive_more::Deref;
use generator::ModelGenerationProject;
use nmsr_rendering::high_level::{model::PlayerModel, types::PlayerPartTextureType};

#[derive(Parser, Debug)]
#[clap(name = env!("CARGO_CRATE_NAME"), version)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long, required = false, value_enum)]
    model: PlayerModelArg,

    #[arg(long, default_value = "true")]
    layers: bool,

    #[arg(short, long)]
    output: PathBuf,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum PlayerModelArg {
    Wide,
    Alex,
    Slim,
    Steve,
}

impl Deref for PlayerModelArg {
    type Target = PlayerModel;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Wide | Self::Steve => &PlayerModel::Steve,
            Self::Alex | Self::Slim => &PlayerModel::Alex,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let skin_bytes = fs::read(args.input).context(anyhow!("Failed to read input skin"))?;

    let mut textures = HashMap::new();

    textures.insert(PlayerPartTextureType::Skin, skin_bytes.clone());

    #[cfg(feature = "ears")]
    let ears_features = {
        let skin_image = image::load_from_memory(&skin_bytes)
            .context(anyhow!("Failed to open input skin"))?
            .into_rgba8();

        ears_rs::parser::EarsParser::parse(&skin_image)
            .context(anyhow!("Failed to parse ears features from skin"))?
    };

    let project = ModelGenerationProject::new(
        *args.model,
        args.layers,
        textures,
        #[cfg(feature = "ears")]
        ears_features,
    );

    blockbench::generate_project(project, args.output)
        .context(anyhow!("Failed to generate blockbench project"))?;

    println!("Done!");
        
    Ok(())
}

mod blockbench;
mod generator;

use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{anyhow, Context, Ok, Result};
use clap::{Parser, ValueEnum};
use derive_more::Deref;
use ears_rs::alfalfa::AlfalfaDataKey;
use generator::ModelGenerationProject;
use nmsr_rendering::high_level::{
    model::PlayerModel, parts::provider::ears::PlayerPartEarsTextureType,
    types::PlayerPartTextureType,
};

use crate::blockbench::write_png;

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
        let mut skin_image = image::load_from_memory(&skin_bytes)
            .context(anyhow!("Failed to open input skin"))?
            .into_rgba8();

        let alfalfa = ears_rs::alfalfa::read_alfalfa(&skin_image)?;
        
        
        if let Some(alfalfa) = alfalfa {
            if let Some(wings) = alfalfa.get_data(AlfalfaDataKey::Wings) {
                textures.insert(PlayerPartEarsTextureType::Wings.into(), wings.to_vec());
            }
        }
        
        let features = ears_rs::parser::EarsParser::parse(&skin_image)
        .context(anyhow!("Failed to parse ears features from skin"));
    
        ears_rs::utils::process_erase_regions(&mut skin_image)?;
        ears_rs::utils::strip_alpha(&mut skin_image);
        
        if let Result::Ok(new_skin_bytes) = write_png(&skin_image) {
            textures.insert(PlayerPartTextureType::Skin, new_skin_bytes);
        }

        features
    }?;

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

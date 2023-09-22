use std::ops::Deref;
use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Ok, Result};
use clap::{Parser, ValueEnum};
use nmsr_rendering_blockbench_model_generator_experiment::blockbench;
use nmsr_rendering_blockbench_model_generator_experiment::generator::ModelGenerationProject;
use nmsr_rendering_blockbench_model_generator_experiment::nmsr_rendering::high_level::{
    model::PlayerModel, types::PlayerPartTextureType,
};

#[derive(Parser, Debug)]
#[clap(name = env!("CARGO_CRATE_NAME"), version)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long, required = false, value_enum)]
    model: PlayerModelArg,

    #[arg(long, default_value = "true")]
    layers: bool,

    #[arg(long)]
    open: bool,

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
    std::env::remove_var("ELECTRON_RUN_AS_NODE");
    
    let args = Args::parse();
    let skin_bytes = fs::read(args.input).context(anyhow!("Failed to read input skin"))?;

    let mut project = ModelGenerationProject::new(
        *args.model,
        args.layers,
    );
    
    project.load_texture(PlayerPartTextureType::Skin, &skin_bytes)?;

    let result = blockbench::generate_project(project)
        .context(anyhow!("Failed to generate blockbench project"))?;

    fs::write(&args.output, result).context(anyhow!("Failed to write project to file"))?;
        
    if args.open {
        println!("Opening blockbench project...");
        opener::open(args.output.canonicalize()?)?;
    }

    println!("Done!");

    Ok(())
}

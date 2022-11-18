use anyhow::Result;
use vergen::{Config, vergen};

fn main() -> Result<()> {
    vergen(Config::default())
}

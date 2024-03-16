use std::error::Error;
use vergen_gitcl::{Emitter, GitclBuilder};

fn main() -> Result<(), Box<dyn Error>> {
    // Emit the instructions to the cargo build script (currently, just the current git sha hash)
    Emitter::default().add_instructions(&GitclBuilder::default().sha(true).build()?)?;
    Ok(())
}

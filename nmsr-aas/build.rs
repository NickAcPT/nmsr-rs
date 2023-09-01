use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    // Emit the instructions to the cargo build script (currently, just the current git sha hash)
    EmitBuilder::builder().git_sha(true).emit()?;
    Ok(())
}

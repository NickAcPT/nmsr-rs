use std::error::Error;
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    // Emit the instructions to the cargo build script
    EmitBuilder::builder().git_sha(false).emit()?;
    Ok(())
}
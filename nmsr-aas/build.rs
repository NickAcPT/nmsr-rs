use std::{error::Error, process};

fn main() -> Result<(), Box<dyn Error>> {
    // Emit the instructions to the cargo build script (currently, just the current git sha hash)
    println!("cargo:rerun-if-changed=build.rs");
    
    let result = process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;
    
    if !result.status.success() {
        return Err(format!("Failed to execute git: {}", String::from_utf8_lossy(&result.stderr)).into());
    }
    
    let git_hash = String::from_utf8_lossy(&result.stdout);
    let git_hash = git_hash.trim();
    
    println!("cargo:rustc-env=VERGEN_IS_LITERALLY_TRASH__IT_DOES_NOT_WORK_AND_IT_ACTUALLY_BREAKS_EVERY_TIME_I_UPDATE_IT__LIKE_SERIOUSLY_HOW_IS_THAT_POSSIBLE___STOP_CHANGING_THE_DAMN_IMPLEMENTATION___I_JUST_WANT_A_STUPID_GIT_HASH={}", git_hash);
    
    Ok(())
}

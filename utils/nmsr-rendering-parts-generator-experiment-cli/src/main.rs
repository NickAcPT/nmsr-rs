use std::path::PathBuf;

use anyhow::{Ok, Result};
use nmsr_rendering_parts_generator_experiment::{
    nmsr_rendering::high_level::{
        camera::{Camera, CameraRotation, ProjectionParameters},
        pipeline::scene::{Size, SunInformation},
    },
    PartsGroupLogic,
};

use nmsr_rendering_parts_generator_experiment::generate_parts;

#[pollster::main]
async fn main() -> Result<()> {
    let rotation = CameraRotation {
        yaw: 20.0,
        pitch: 10.0,
        roll: 0.0,
    };

    let camera = Camera::new_orbital(
        [0.0, 16.5, 0.0].into(),
        45.0,
        rotation,
        ProjectionParameters::Perspective { fov: 45.0 },
        None,
    );

    let sun = SunInformation::new([0.0, -1.0, 5.0].into(), 1.0, 0.621);

    let viewport_size = Size {
        width: 512,
        height: 832,
    };

    generate_parts(
        camera,
        sun,
        viewport_size,
        PartsGroupLogic::MergeEverything,
        None,
        PathBuf::from("renders"),
    )
    .await?;

    Ok(())
}

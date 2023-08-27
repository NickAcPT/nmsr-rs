use glam::{Mat4, Quat, Vec3};
use std::f32::consts;

pub(crate) static FLIP_X_AND_Z: Vec3 = Vec3::new(-1.0, 1.0, -1.0);

pub(crate) fn minecraft_rotation_matrix(yaw: f32, pitch: f32, roll: f32) -> Mat4 {
    Mat4::from_scale(FLIP_X_AND_Z) * 
        Mat4::from_quat(Quat::from_euler(
            glam::EulerRot::ZXY,
            roll.to_radians(),
            -pitch.to_radians(),
            yaw.to_radians(),
        ))
}

pub(crate) fn look_from_yaw_pitch(yaw: f32, pitch: f32) -> Vec3 {
    let (y_sin, y_cos) = f32::sin_cos((-yaw).to_radians() - consts::PI);
    let (p_sin, p_cos) = f32::sin_cos((-pitch).to_radians());

    let x = y_sin * p_cos;
    let y = p_sin;
    let z = y_cos * p_cos;

    Vec3::new(x, y, z) * FLIP_X_AND_Z
}

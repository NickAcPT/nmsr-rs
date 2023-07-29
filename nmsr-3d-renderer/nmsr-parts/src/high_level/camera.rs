use std::f32::consts;
use std::time::SystemTime;

use glam::{Mat4, Vec3};

pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}

/// The camera used to view the scene
pub struct Camera {
    /// The position of the camera
    pub position: Vec3,
    /// The rotation of the camera
    pub rotation: CameraRotation,
    /// The field of view of the camera (in degrees)
    pub fov: f32,
    pub amogus: SystemTime,
}

impl Camera {
    pub fn new(position: Vec3, rotation: CameraRotation, fov: f32) -> Self {
        Camera {
            position,
            rotation,
            fov,
            amogus: SystemTime::now(),
        }
    }

    pub fn set_x(&mut self, x: f32) {
        self.position.x = x;
    }

    pub fn set_y(&mut self, y: f32) {
        self.position.y = y;
    }

    pub fn set_z(&mut self, z: f32) {
        self.position.z = z;
    }

    pub fn set_yaw(&mut self, yaw: f32) {
        self.rotation.yaw = yaw;
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.rotation.pitch = pitch;
    }

    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
    }

    pub fn generate_view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let projection = Mat4::perspective_rh(self.fov.to_radians(), aspect_ratio, 1.0, 100.0);
        let view = Self::minecraft_rotation_matrix(self, self.rotation.yaw, self.rotation.pitch);
        let position = Mat4::from_translation(-self.position);

        projection * view * position
    }

    fn minecraft_rotation_matrix(&self, yaw: f32, pitch: f32) -> Mat4 {
        let (y_sin, y_cos) = f32::sin_cos((-yaw).to_radians() - consts::PI);
        let (p_sin, p_cos) = f32::sin_cos((-pitch).to_radians());

        let x = y_sin * p_cos;
        let y = p_sin;
        let z = y_cos * p_cos;

        let look = Vec3::new(x, y, z);
        let flip_x_and_z = Vec3::new(-1.0, 1.0, -1.0);

        Mat4::look_at_rh(Vec3::ZERO, look * flip_x_and_z, Vec3::Y)
    }
}
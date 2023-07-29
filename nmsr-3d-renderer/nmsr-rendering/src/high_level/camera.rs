use crate::high_level::utils::{camera_getters_setters, camera_inner_getters_setters};
use crate::low_level::utils::minecraft_rotation_matrix;
use glam::{Mat4, Vec3};

pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}

/// The camera used to view the scene
pub struct Camera {
    /// The position of the camera
    position: Vec3,
    /// The rotation of the camera
    rotation: CameraRotation,
    /// The field of view of the camera (in degrees)
    fov: f32,
    /// The aspect ratio of the camera
    aspect_ratio: f32,

    dirty: bool,
    cached_view_projection_matrix: Mat4,
}

impl Camera {
    pub fn new(position: Vec3, rotation: CameraRotation, fov: f32, aspect_ratio: f32) -> Self {
        Camera {
            position,
            rotation,
            fov,
            aspect_ratio,
            dirty: true,
            cached_view_projection_matrix: Mat4::ZERO,
        }
    }

    camera_getters_setters!(
        fov: f32,
        position: Vec3,
        rotation: CameraRotation,
        aspect_ratio: f32
    );

    camera_inner_getters_setters!(position, x, y, z);
    camera_inner_getters_setters!(rotation, yaw, pitch);

    pub fn get_view_projection_matrix(&mut self) -> Mat4 {
        if self.dirty {
            self.cached_view_projection_matrix = self.compute_view_projection_matrix()
        }

        self.cached_view_projection_matrix
    }

    fn compute_view_projection_matrix(&self) -> Mat4 {
        let projection = Mat4::perspective_rh(self.fov.to_radians(), self.aspect_ratio, 0.1, 100.0);
        let view = minecraft_rotation_matrix(self.rotation.yaw, self.rotation.pitch);
        let position = Mat4::from_translation(-self.position);

        projection * view * position
    }
}

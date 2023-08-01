use glam::{Mat4, Vec3};

use crate::high_level::utils::{camera_getters_setters, camera_inner_getters_setters};
use crate::low_level::utils::minecraft_rotation_matrix;

#[derive(Copy, Clone)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Copy, Clone)]
pub enum ProjectionParameters {
    Perspective {
        /// The field of view of the camera (in degrees)
        fov: f32,
    },
    Orthographic {
        /// The width of the camera
        aspect: f32,
    },
}

impl PartialEq for ProjectionParameters {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl ProjectionParameters {

    pub fn get_fov(&self) -> Option<f32> {
        match self {
            ProjectionParameters::Perspective { fov } => Some(*fov),
            _ => None,
        }
    }

    pub fn get_aspect(&self) -> Option<f32> {
        match self {
            ProjectionParameters::Orthographic { aspect } => Some(*aspect),
            _ => None,
        }
    }

    pub fn as_mut_fov(&mut self) -> Option<&mut f32> {
        match self {
            ProjectionParameters::Perspective { ref mut fov } => Some(fov),
            _ => None,
        }
    }

    pub fn as_mut_aspect(&mut self) -> Option<&mut f32> {
        match self {
            ProjectionParameters::Orthographic { ref mut aspect } => Some(aspect),
            _ => None,
        }
    }

    fn compute_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        match self {
            ProjectionParameters::Perspective { fov } => {
                Mat4::perspective_rh(fov.to_radians(), aspect_ratio, 0.1, 100.0)
            }
            ProjectionParameters::Orthographic { aspect } => {
                Mat4::orthographic_rh(
                    -aspect * aspect_ratio,
                    aspect * aspect_ratio,
                    -*aspect,
                    *aspect,
                    0.1,
                    100.0)
            }
        }
    }

}

/// The camera used to view the scene
pub struct Camera {
    /// The position of the camera
    position: Vec3,
    /// The rotation of the camera
    rotation: CameraRotation,
    /// The aspect ratio of the camera
    aspect_ratio: f32,
    projection: ProjectionParameters,

    dirty: bool,
    cached_view_projection_matrix: Mat4,
}

impl Camera {
    pub fn new(position: Vec3, rotation: CameraRotation, projection: ProjectionParameters, aspect_ratio: f32) -> Self {
        Camera {
            position,
            rotation,
            aspect_ratio,
            projection,
            dirty: true,
            cached_view_projection_matrix: Mat4::ZERO,
        }
    }

    camera_getters_setters!(
        position: Vec3,
        rotation: CameraRotation,
        aspect_ratio: f32,
        projection: ProjectionParameters
    );

    camera_inner_getters_setters!(position, x, y, z);
    camera_inner_getters_setters!(rotation, yaw, pitch);

    pub fn get_fov(&self) -> f32 {
        self.projection.get_fov().unwrap_or(0f32)
    }
    pub fn set_fov(&mut self, fov: f32) {
        if let Some(fov_mut) = self.projection.as_mut_fov() {
            *fov_mut = fov;
        }
        self.dirty = true;
    }
    pub fn get_aspect(&self) -> f32 {
        self.projection.get_aspect().unwrap_or(0f32)
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        if let Some(aspect_mut) = self.projection.as_mut_aspect() {
            *aspect_mut = aspect;
        }
        self.dirty = true;
    }

    pub fn get_view_projection_matrix(&mut self) -> Mat4 {
        if self.dirty {
            self.cached_view_projection_matrix = self.compute_view_projection_matrix()
        }

        self.cached_view_projection_matrix
    }

    fn compute_view_projection_matrix(&self) -> Mat4 {
        let projection = self.projection.compute_projection_matrix(self.aspect_ratio);
        let view = minecraft_rotation_matrix(self.rotation.yaw, self.rotation.pitch);
        let position = Mat4::from_translation(-self.position);

        projection * view * position
    }
}

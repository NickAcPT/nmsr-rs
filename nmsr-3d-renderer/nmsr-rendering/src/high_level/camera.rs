use glam::{Mat4, Vec3};
use std::mem;

use crate::high_level::utils::{
    camera_getters_setters, camera_inner_getters_setters, camera_inner_getters_setters_opt,
};
use crate::low_level::utils::{look_from_yaw_pitch, minecraft_rotation_matrix};

use super::pipeline::scene::Size;

#[derive(Copy, Clone, Debug)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

impl CameraRotation {
    pub fn create_rotation_matrix(&self) -> Mat4 {
        minecraft_rotation_matrix(self.yaw, self.pitch, self.roll)
    }
}

impl core::ops::Neg for CameraRotation {
    type Output = Self;

    fn neg(self) -> Self::Output {
        CameraRotation {
            yaw: -self.yaw,
            pitch: -self.pitch,
            roll: -self.roll,
        }
    }
}

impl core::ops::Add for CameraRotation {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        CameraRotation {
            yaw: self.yaw + rhs.yaw,
            pitch: self.pitch + rhs.pitch,
            roll: self.roll + rhs.roll,
        }
    }
}

impl core::ops::AddAssign for CameraRotation {
    fn add_assign(&mut self, rhs: Self) {
        self.yaw += rhs.yaw;
        self.pitch += rhs.pitch;
        self.roll += rhs.roll;
    }
}

#[derive(Copy, Clone, Debug)]
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
            ProjectionParameters::Orthographic { aspect } => Mat4::orthographic_rh(
                -aspect * aspect_ratio,
                aspect * aspect_ratio,
                -*aspect,
                *aspect,
                -100.0,
                100.0,
            ),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum CameraPositionParameters {
    Absolute(Vec3),
    Orbital {
        /// The look at point of the camera
        look_at: Vec3,
        /// The distance from the camera to the look at point
        distance: f32,
    },
}

impl PartialEq for CameraPositionParameters {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }
}

impl CameraPositionParameters {
    pub fn get_position(&self) -> Option<Vec3> {
        match self {
            CameraPositionParameters::Absolute(position) => Some(*position),
            CameraPositionParameters::Orbital { .. } => None,
        }
    }

    pub fn get_look_at(&self) -> Option<Vec3> {
        match self {
            CameraPositionParameters::Absolute(_) => None,
            CameraPositionParameters::Orbital { look_at, .. } => Some(*look_at),
        }
    }

    pub fn get_distance(&self) -> Option<f32> {
        match self {
            CameraPositionParameters::Absolute(_) => None,
            CameraPositionParameters::Orbital { distance, .. } => Some(*distance),
        }
    }

    pub fn as_mut_position(&mut self) -> Option<&mut Vec3> {
        match self {
            CameraPositionParameters::Absolute(ref mut position) => Some(position),
            CameraPositionParameters::Orbital { .. } => None,
        }
    }

    pub fn as_mut_look_at(&mut self) -> Option<&mut Vec3> {
        match self {
            CameraPositionParameters::Absolute(_) => None,
            CameraPositionParameters::Orbital {
                ref mut look_at, ..
            } => Some(look_at),
        }
    }

    pub fn as_mut_distance(&mut self) -> Option<&mut f32> {
        match self {
            CameraPositionParameters::Absolute(_) => None,
            CameraPositionParameters::Orbital {
                ref mut distance, ..
            } => Some(distance),
        }
    }

    pub fn to_absolute(&self, yaw: f32, pitch: f32) -> Self {
        match self {
            CameraPositionParameters::Absolute(_) => *self,
            CameraPositionParameters::Orbital { look_at, distance } => {
                // Look pos is a vector pointing in the direction the camera is looking (from the origin)
                let look_pos = look_from_yaw_pitch(yaw, pitch);
                // To get the position of the camera, we take the point where we want to look,
                // and move backwards along the look pos vector by the distance we want to be from the look at point
                let pos = *look_at + (-look_pos * *distance);

                CameraPositionParameters::Absolute(pos)
            }
        }
    }
}

/// The camera used to view the scene
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// The position of the camera
    position_parameters: CameraPositionParameters,
    /// The rotation of the camera
    rotation: CameraRotation,
    /// The aspect ratio of the camera
    size: Option<Size>,
    projection: ProjectionParameters,

    dirty: bool,
    cached_view_projection_matrix: Mat4,
}

impl Camera {
    pub fn new_absolute(
        position: Vec3,
        rotation: CameraRotation,
        projection: ProjectionParameters,
        size: Option<Size>,
    ) -> Self {
        Camera {
            position_parameters: CameraPositionParameters::Absolute(position),
            rotation,
            size,
            projection,
            dirty: true,
            cached_view_projection_matrix: Mat4::ZERO,
        }
    }

    pub fn new_orbital(
        look_at: Vec3,
        distance: f32,
        rotation: CameraRotation,
        projection: ProjectionParameters,
        size: Option<Size>,
    ) -> Self {
        Camera {
            position_parameters: CameraPositionParameters::Orbital { look_at, distance },
            rotation,
            size,
            projection,
            dirty: true,
            cached_view_projection_matrix: Mat4::ZERO,
        }
    }

    pub fn get_aspect_ratio(&self) -> f32 {
        self.size
            .map(|size| size.width as f32 / size.height as f32)
            .unwrap_or(1.0)
    }

    camera_getters_setters!(
        position_parameters: CameraPositionParameters,
        rotation: CameraRotation,
        size: Option<Size>,
        projection: ProjectionParameters
    );

    camera_inner_getters_setters!(rotation, yaw, pitch, roll);
    camera_inner_getters_setters_opt!(projection, fov, aspect);
    camera_inner_getters_setters_opt!(position_parameters, position: Vec3, Vec3::ZERO);
    camera_inner_getters_setters_opt!(position_parameters, look_at: Vec3, Vec3::ZERO);
    camera_inner_getters_setters_opt!(position_parameters, distance);

    camera_inner_getters_setters!(get_position(), position, x, y, z);
    camera_inner_getters_setters!(get_look_at(), look_at, x, y, z);

    pub fn get_view_projection_matrix(&mut self) -> Mat4 {
        if self.dirty {
            self.cached_view_projection_matrix = self.compute_view_projection_matrix()
        }

        self.cached_view_projection_matrix
    }

    fn compute_view_projection_matrix(&self) -> Mat4 {
        let projection = self
            .projection
            .compute_projection_matrix(self.get_aspect_ratio());

        let view_position = match self.position_parameters {
            CameraPositionParameters::Absolute(pos) => {
                let view = minecraft_rotation_matrix(
                    self.rotation.yaw,
                    self.rotation.pitch,
                    self.rotation.roll,
                );
                let position = Mat4::from_translation(-pos);

                view * position
            }
            CameraPositionParameters::Orbital { look_at, distance } => {
                // Look pos is a vector pointing in the direction the camera is looking (from the origin)
                let look_pos = look_from_yaw_pitch(self.rotation.yaw, self.rotation.pitch);
                // To get the position of the camera, we take the point where we want to look,
                // and move backwards along the look pos vector by the distance we want to be from the look at point
                let pos = look_at + (-look_pos * distance);

                // Compute roll matrix
                let roll_matrix = Mat4::from_rotation_z(-self.rotation.roll.to_radians());

                roll_matrix * Mat4::look_at_rh(pos, look_at, Vec3::Y)
            }
        };

        projection * view_position
    }
}

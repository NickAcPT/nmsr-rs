use std::f32::consts;
use std::mem;

use nmsr_rendering::low_level::{EulerRot, Mat4, Quat, Vec3};

use crate::model::Size;

pub static FLIP_X_AND_Z: Vec3 = Vec3::new(-1.0, 1.0, -1.0);

pub fn minecraft_rotation_matrix(yaw: f32, pitch: f32, roll: f32) -> Mat4 {
    Mat4::from_scale(FLIP_X_AND_Z)
        * Mat4::from_quat(Quat::from_euler(
            EulerRot::ZXY,
            roll.to_radians(),
            -pitch.to_radians(),
            yaw.to_radians(),
        ))
}

pub fn look_from_yaw_pitch(yaw: f32, pitch: f32) -> Vec3 {
    let (y_sin, y_cos) = f32::sin_cos((-yaw).to_radians() - consts::PI);
    let (p_sin, p_cos) = f32::sin_cos((-pitch).to_radians());

    let x = y_sin * p_cos;
    let y = p_sin;
    let z = y_cos * p_cos;

    Vec3::new(x, y, z) * FLIP_X_AND_Z
}

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

    pub fn get_size(&self) -> Option<Size> {
        self.size
    }
    
    pub fn set_size(&mut self, size: Option<Size>) {
        self.size = size;
        self.dirty = true;
    }

    pub fn get_aspect_ratio(&self) -> f32 {
        self.size
            .map(|size| size.width as f32 / size.height as f32)
            .unwrap_or(1.0)
    }

    pub fn get_cached_view_projection_matrix(&self) -> Mat4 {
        self.cached_view_projection_matrix
    }

    pub fn update_mvp(&mut self) {
        self.dirty = false;
        self.cached_view_projection_matrix = self.compute_view_projection_matrix();
    }

    pub fn get_view_projection_matrix(&mut self) -> Mat4 {
        self.update_mvp();

        self.cached_view_projection_matrix
    }

    pub fn get_distance(&self) -> f32 {
        self.position_parameters.get_distance().unwrap_or(0.0)
    }
    
    pub fn get_distance_as_mut(&mut self) -> Option<&mut f32> {
        self.position_parameters.as_mut_distance()
    }

    pub fn get_rotation(&self) -> CameraRotation {
        self.rotation
    }

    pub fn get_rotation_mut(&mut self) -> &mut CameraRotation {
        &mut self.rotation
    }
    
    pub fn set_rotation(&mut self, rotation: CameraRotation) {
        *self.get_rotation_mut() = rotation;
    }
    
    #[inline]
    pub fn get_rotation_as_mut(&mut self) -> &mut CameraRotation {
        self.get_rotation_mut()
    }
    
    pub fn get_yaw(&self) -> f32 {
        self.get_rotation().yaw
    }
    
    pub fn get_pitch(&self) -> f32 {
        self.get_rotation().pitch
    }
    
    pub fn get_roll(&self) -> f32 {
        self.get_rotation().roll
    }
    
    pub fn get_position_parameters_mut(&mut self) -> &mut CameraPositionParameters {
        &mut self.position_parameters
    }
    
    pub fn get_projection_mut(&mut self) -> &mut ProjectionParameters {
        &mut self.projection
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

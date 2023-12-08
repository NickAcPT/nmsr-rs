use super::provider::minecraft::compute_base_part;
use crate::parts::part::Part::{Cube, Quad};
use crate::parts::uv::{CubeFaceUvs, FaceUv};
use crate::types::{PlayerBodyPartType, PlayerPartTextureType};
use glam::{Mat4, Quat, Vec3};

#[cfg(feature = "part_tracker")]
use super::tracking::PartTrackingData;

#[derive(Debug, Copy, Clone)]
pub struct PartAnchorInfo {
    pub rotation_anchor: MinecraftPosition,
    pub translation_anchor: MinecraftPosition,
}
impl PartAnchorInfo {
    pub fn with_rotation_anchor(mut self, rotation_anchor: MinecraftPosition) -> Self {
        self.rotation_anchor += rotation_anchor;
        self
    }

    pub fn with_translation_anchor(mut self, translation_anchor: MinecraftPosition) -> Self {
        self.translation_anchor += translation_anchor;
        self
    }

    pub fn without_translation_anchor(mut self) -> Self {
        self.translation_anchor = MinecraftPosition::ZERO;
        self
    }

    pub fn new_rotation_anchor_position(rotation_anchor: MinecraftPosition) -> Self {
        Self {
            rotation_anchor,
            translation_anchor: MinecraftPosition::ZERO,
        }
    }

    pub fn new_part_anchor_translate(part: PlayerBodyPartType, slim_arms: bool) -> Self {
        let part = compute_base_part(part, slim_arms);

        let pos = part.get_position();
        let size = part.get_size();

        let translation_anchor: Vec3 = [pos.x, pos.y, pos.z].into();

        Self {
            rotation_anchor: translation_anchor,
            translation_anchor,
        }
    }
}

impl Default for PartAnchorInfo {
    fn default() -> Self {
        Self {
            rotation_anchor: MinecraftPosition::ZERO,
            translation_anchor: MinecraftPosition::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Part {
    /// Represents a cube as a part of a player model.
    Cube {
        position: MinecraftPosition,
        size: MinecraftPosition,
        rotation_matrix: Mat4,
        face_uvs: CubeFaceUvs,
        texture: PlayerPartTextureType,
        #[cfg(feature = "part_tracker")]
        part_tracking_data: PartTrackingData,
    },
    /// Represents a quad as a part of a player model.
    Quad {
        position: MinecraftPosition,
        size: MinecraftPosition,
        rotation_matrix: Mat4,
        face_uv: FaceUv,
        normal: Vec3,
        texture: PlayerPartTextureType,
        #[cfg(feature = "part_tracker")]
        part_tracking_data: PartTrackingData,
    },
}

impl Part {
    /// Creates a new cube part.
    ///
    /// # Arguments
    ///
    /// * `pos`: The position of the cube. [x, y, z]
    /// * `size`: The size of the cube. [x, y, z]
    /// * `uvs`: The UVs of the cube.
    /// UVs are in the following order: [North, South, East, West, Up, Down]
    /// Each UV is in the following order: [Top left, Bottom right]
    ///
    /// returns: [Part]
    pub fn new_cube(
        texture: PlayerPartTextureType,
        pos: [i32; 3],
        size: [u32; 3],
        uvs: CubeFaceUvs,
        #[cfg(feature = "part_tracker")] name: Option<String>,
    ) -> Self {
        Cube {
            position: MinecraftPosition::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
            size: MinecraftPosition::new(size[0] as f32, size[1] as f32, size[2] as f32),
            rotation_matrix: Mat4::IDENTITY,
            face_uvs: uvs,
            texture,
            #[cfg(feature = "part_tracker")]
            part_tracking_data: PartTrackingData::new(name),
        }
    }

    pub fn new_quad(
        texture: PlayerPartTextureType,
        pos: [f32; 3],
        size: [u32; 3],
        uvs: FaceUv,
        normal: Vec3,
        #[cfg(feature = "part_tracker")] name: Option<String>,
    ) -> Self {
        Quad {
            position: MinecraftPosition::new(pos[0], pos[1], pos[2]),
            size: MinecraftPosition::new(size[0] as f32, size[1] as f32, size[2] as f32),
            rotation_matrix: Mat4::IDENTITY,
            face_uv: uvs,
            normal,
            texture,
            #[cfg(feature = "part_tracker")]
            part_tracking_data: PartTrackingData::new(name),
        }
    }

    pub fn expand_splat(&self, amount: f32) -> Self {
        self.expand(Vec3::splat(amount))
    }

    pub fn expand(&self, amount: Vec3) -> Self {
        let mut new_part = self.clone();
        let amount = amount * 2.0;

        match new_part {
            Cube {
                ref mut size,
                ref mut position,
                ..
            } => {
                // Increase the size of the cube by the amount specified.
                *size += amount;

                // Fix the position of the cube so that it is still centered.
                *position -= amount / 2.0;
            }
            Quad {
                ref mut size,
                ref mut position,
                ..
            } => {
                // Increase the size of the quad by the amount specified.
                *size += amount;
            }
        }

        new_part
    }

    pub fn get_rotation_matrix(&self) -> Mat4 {
        match self {
            Cube {
                rotation_matrix, ..
            } => *rotation_matrix,
            Quad {
                rotation_matrix, ..
            } => *rotation_matrix,
        }
    }

    fn rotation_matrix_mut(&mut self) -> &mut Mat4 {
        match self {
            Cube {
                rotation_matrix, ..
            } => rotation_matrix,
            Quad {
                rotation_matrix, ..
            } => rotation_matrix,
        }
    }

    #[cfg(feature = "part_tracker")]
    pub fn last_rotation(&self) -> Option<(MinecraftPosition, PartAnchorInfo)> {
        match self {
            Cube {
                part_tracking_data, ..
            } => part_tracking_data.last_rotation().copied(),
            Quad {
                part_tracking_data, ..
            } => part_tracking_data.last_rotation().copied(),
        }
    }

    #[cfg(feature = "part_tracker")]
    fn last_rotation_mut(&mut self) -> &mut Option<(MinecraftPosition, PartAnchorInfo)> {
        match self {
            Cube {
                part_tracking_data, ..
            } => part_tracking_data.last_rotation_mut(),
            Quad {
                part_tracking_data, ..
            } => part_tracking_data.last_rotation_mut(),
        }
    }

    pub fn translate(&mut self, translation: MinecraftPosition) {
        *self.position_mut() += translation;
    }

    pub fn rotate(&mut self, rotation: MinecraftPosition, anchor: Option<PartAnchorInfo>) {
        let prev_rotation = *self.rotation_matrix_mut();

        let anchor = anchor.unwrap_or_default();
        *self.position_mut() += anchor.translation_anchor;

        let offset = anchor.rotation_anchor;

        let rot_translation_mat = Mat4::from_translation(offset);
        let neg_rot_translation_mat = Mat4::from_translation(-offset);

        let rotation_mat = Mat4::from_quat(Quat::from_euler(
            glam::EulerRot::YXZ,
            rotation.y.to_radians(),
            -rotation.x.to_radians(),
            -rotation.z.to_radians(),
        ));

        let model_transform = rot_translation_mat * rotation_mat * neg_rot_translation_mat;

        #[cfg(feature = "part_tracker")]
        if rotation != Vec3::ZERO {
            self.last_rotation_mut().replace((rotation, anchor));
        }

        *self.rotation_matrix_mut() = model_transform * prev_rotation;
    }

    pub fn get_size(&self) -> MinecraftPosition {
        match self {
            Cube { size, .. } => *size,
            Quad { size, .. } => *size,
        }
    }

    pub fn size_mut(&mut self) -> &mut MinecraftPosition {
        match self {
            Cube { size, .. } => size,
            Quad { size, .. } => size,
        }
    }

    pub fn get_position(&self) -> MinecraftPosition {
        match self {
            Cube { position, .. } => *position,
            Quad { position, .. } => *position,
        }
    }

    pub fn position_mut(&mut self) -> &mut MinecraftPosition {
        match self {
            Cube { position, .. } => position,
            Quad { position, .. } => position,
        }
    }

    pub fn get_texture(&self) -> PlayerPartTextureType {
        match self {
            Cube { texture, .. } => *texture,
            Quad { texture, .. } => *texture,
        }
    }

    pub fn set_texture(&mut self, texture: PlayerPartTextureType) {
        match self {
            Cube {
                texture: ref mut t, ..
            } => *t = texture,
            Quad {
                texture: ref mut t, ..
            } => *t = texture,
        }
    }

    pub fn get_face_uv(&self) -> FaceUv {
        match self {
            Cube { face_uvs, .. } => unimplemented!("Cannot get face UV on a cube"),
            Quad { face_uv, .. } => *face_uv,
        }
    }

    pub fn set_face_uv(&mut self, face_uv: FaceUv) {
        match self {
            Cube {
                face_uvs: ref mut f,
                ..
            } => unreachable!("Cannot set face UV on a cube"),
            Quad {
                face_uv: ref mut f, ..
            } => *f = face_uv,
        }
    }

    pub fn get_face_uvs(&self) -> CubeFaceUvs {
        match self {
            Cube { face_uvs, .. } => *face_uvs,
            Quad { face_uv, .. } => unimplemented!("Cannot get face UVs on a quad"),
        }
    }

    pub fn set_face_uvs(&mut self, face_uvs: CubeFaceUvs) {
        match self {
            Cube {
                face_uvs: ref mut f,
                ..
            } => *f = face_uvs,
            Quad {
                face_uv: ref mut f, ..
            } => unreachable!("Cannot set face UVs on a quad"),
        }
    }

    pub fn normal_mut(&mut self) -> &mut Vec3 {
        match self {
            Cube { .. } => unreachable!("Cannot get normal on a cube"),
            Quad { normal, .. } => normal,
        }
    }

    #[cfg(feature = "part_tracker")]
    pub fn part_tracking_data(&self) -> &PartTrackingData {
        match self {
            Cube {
                part_tracking_data, ..
            } => part_tracking_data,
            Quad {
                part_tracking_data, ..
            } => part_tracking_data,
        }
    }
    
    #[cfg(feature = "part_tracker")]
    pub fn part_tracking_data_mut(&mut self) -> &mut PartTrackingData {
        match self {
            Cube {
                part_tracking_data, ..
            } => part_tracking_data,
            Quad {
                part_tracking_data, ..
            } => part_tracking_data,
        }
    }
    

    #[cfg(feature = "part_tracker")]
    pub fn get_name(&self) -> Option<&str> {
        self.part_tracking_data().name().map(String::as_str)
    }

    #[cfg(feature = "part_tracker")]
    pub fn get_group(&self) -> &[String] {
        self.part_tracking_data().group()
    }

    #[cfg(feature = "part_tracker")]
    pub fn push_group(&mut self, group: impl Into<String>) {
        self.part_tracking_data_mut().push_group(group.into());
    }

    #[cfg(feature = "part_tracker")]
    pub fn push_groups(&mut self, group: &[String]) {
        self.part_tracking_data_mut().push_groups(group.into());
    }

    #[cfg(feature = "part_tracker")]
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.push_group(group);

        self
    }

    #[cfg(feature = "markers")]
    pub fn add_marker(&mut self, marker: super::tracking::Marker) {
        self.part_tracking_data_mut().push_marker(marker);
    }

    #[cfg(feature = "markers")]
    pub fn add_markers(&mut self, markers: &[super::tracking::Marker]) {
        self.part_tracking_data_mut().push_markers(markers.into());
    }

    #[cfg(feature = "markers")]
    pub fn markers(&self) -> &[super::tracking::Marker] {
        self.part_tracking_data().markers()
    }
}

/// A position in 3D space.
///
/// Minecraft coordinates are structured as follows:
/// - +X is east / -X is west
/// - +Y is up / -Y is down
/// - +Z is south / -Z is north
pub(crate) type MinecraftPosition = Vec3;

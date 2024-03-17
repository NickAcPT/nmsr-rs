use super::provider::minecraft::compute_base_part;
use crate::parts::part::Part::{Cube, Quad};
use crate::parts::uv::{CubeFaceUvs, FaceUv};
use crate::types::{PlayerBodyPartType, PlayerPartTextureType};
use glam::{Affine3A, Quat, Vec3};

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
        transformation: Affine3A,
        face_uvs: CubeFaceUvs,
        texture: PlayerPartTextureType,
        #[cfg(feature = "part_tracker")]
        part_tracking_data: PartTrackingData,
    },
    /// Represents a quad as a part of a player model.
    Quad {
        transformation: Affine3A,
        size: Vec3,
        face_uv: FaceUv,
        normal: Vec3,
        texture: PlayerPartTextureType,
        #[cfg(feature = "part_tracker")]
        part_tracking_data: PartTrackingData,
    },
    /// Represents a group of parts as a part of a player model.
    /// This is used to group parts together so that they can be rotated together.
    Group {
        parts: Vec<Part>,
        transformation: Affine3A,
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
        let transform = Affine3A::from_scale_rotation_translation(
            Vec3::new(size[0] as f32, size[1] as f32, size[2] as f32),
            Quat::IDENTITY,
            Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
        );

        Cube {
            transformation: transform,
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
        let transform = Affine3A::from_scale_rotation_translation(
            Vec3::ONE,
            Quat::IDENTITY,
            Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
        );

        Quad {
            transformation: transform,
            size: Vec3::new(size[0] as f32, size[1] as f32, size[2] as f32),
            face_uv: uvs,
            normal,
            texture,
            #[cfg(feature = "part_tracker")]
            part_tracking_data: PartTrackingData::new(name),
        }
    }

    pub fn new_group(
        texture: PlayerPartTextureType,
        parts: Vec<Part>,
        #[cfg(feature = "part_tracker")] name: Option<String>,
    ) -> Self {
        Self::Group {
            parts,
            transformation: Affine3A::IDENTITY,
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
        let amount_doubled = amount * 2.0;

        match new_part {
            Quad { ref mut size, .. } => {
                *size += amount_doubled;
            }
            Part::Group {
                ref mut transformation,
                ..
            }
            | Part::Cube {
                ref mut transformation,
                ..
            } => {
                let (mut scale, rot, mut trans) = transformation.to_scale_rotation_translation();

                scale += amount_doubled;
                trans -= amount;

                *new_part.transformation_mut() = Affine3A::from_scale_rotation_translation(scale, rot, trans);
            }
        }

        new_part
    }

    pub fn transform_affine(&mut self, t: Affine3A) {
        *self.transformation_mut() = t * self.get_transformation();
        
        if let Self::Quad { normal, .. } = self {
            *normal = t.transform_vector3(*normal);
        }
    }

    pub fn get_transformation(&self) -> Affine3A {
        match self {
            Self::Cube { transformation, .. } => *transformation,
            Self::Quad { transformation, .. } => *transformation,
            Self::Group { transformation, .. } => *transformation,
        }
    }

    fn transformation_mut(&mut self) -> &mut Affine3A {
        match self {
            Self::Cube { transformation, .. } => transformation,
            Self::Quad { transformation, .. } => transformation,
            Self::Group { transformation, .. } => transformation,
        }
    }

    pub fn translate(&mut self, translation: MinecraftPosition) {
        self.transform_affine(Affine3A::from_translation(translation));
    }

    pub fn rotate(&mut self, rotation: MinecraftPosition, anchor: Option<PartAnchorInfo>) {
        let rotation_quat = Quat::from_euler(
            glam::EulerRot::YXZ,
            rotation.y.to_radians(),
            rotation.x.to_radians(),
            rotation.z.to_radians(),
        );

        let mut result = Affine3A::IDENTITY;

        if let Some(anchor) = anchor {
            result *= Affine3A::from_translation(anchor.translation_anchor);
            result *= Affine3A::from_translation(anchor.rotation_anchor);
            
            
            #[cfg(feature = "part_tracker")]
            {
                self.part_tracking_data_mut().set_last_rotation_origin(anchor.rotation_anchor);
            }
        }
        
        result *= Affine3A::from_quat(rotation_quat);

        if let Some(anchor) = anchor {
            result *= Affine3A::from_translation(-anchor.rotation_anchor);
        }
        
        self.transform_affine(result)
    }

    pub fn get_size(&self) -> MinecraftPosition {
        match self {
            Self::Cube { transformation, .. } => transformation.to_scale_rotation_translation().0,
            Self::Quad { size, .. } => *size,
            Self::Group { parts, .. } => {
                let mut min = MinecraftPosition::new(f32::MAX, f32::MAX, f32::MAX);
                let mut max = MinecraftPosition::new(f32::MIN, f32::MIN, f32::MIN);

                for part in parts {
                    let pos = part.get_position();
                    let size = part.get_size();

                    min = min.min(pos);
                    max = max.max(pos + size);
                }

                max - min
            }
        }
    }

    pub fn get_position(&self) -> MinecraftPosition {
        match self {
            Self::Cube { transformation, .. } => transformation.to_scale_rotation_translation().2,
            Self::Quad { transformation, .. } => transformation.to_scale_rotation_translation().2,
            Self::Group { .. } => unreachable!("Cannot get position on a group"),
        }
    }

    pub fn get_texture(&self) -> PlayerPartTextureType {
        match self {
            Cube { texture, .. } => *texture,
            Quad { texture, .. } => *texture,
            Self::Group { texture, .. } => *texture,
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
            Self::Group {
                texture: ref mut t, ..
            } => *t = texture,
        }
    }

    pub fn get_face_uv(&self) -> FaceUv {
        match self {
            Cube { face_uvs, .. } => unimplemented!("Cannot get face UV on a cube"),
            Quad { face_uv, .. } => *face_uv,
            Self::Group { .. } => unreachable!("Cannot get face UV on a group"),
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
            Self::Group { .. } => unreachable!("Cannot set face UV on a group"),
        }
    }

    pub fn get_face_uvs(&self) -> CubeFaceUvs {
        match self {
            Cube { face_uvs, .. } => *face_uvs,
            Quad { face_uv, .. } => unimplemented!("Cannot get face UVs on a quad"),
            Self::Group { .. } => unreachable!("Cannot get face UVs on a group"),
        }
    }

    pub fn set_face_uvs(&mut self, face_uvs: CubeFaceUvs) {
        match self {
            Cube {
                face_uvs: ref mut f,
                ..
            } => *f = face_uvs,
            Self::Quad { .. } => unreachable!("Cannot set face UVs on a quad"),
            Self::Group { .. } => unreachable!("Cannot set face UVs on a group"),
        }
    }

    pub fn normal_mut(&mut self) -> &mut Vec3 {
        match self {
            Cube { .. } => unreachable!("Cannot get normal on a cube"),
            Quad { normal, .. } => normal,
            Self::Group { .. } => unreachable!("Cannot get normal on a group"),
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
            Self::Group {
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
            Self::Group {
                part_tracking_data, ..
            } => part_tracking_data,
        }
    }

    #[cfg(feature = "part_tracker")]
    pub fn get_name(&self) -> Option<&str> {
        self.part_tracking_data().name().map(String::as_str)
    }
    
    #[cfg(feature = "part_tracker")]
    pub fn get_name_mut(&mut self) -> &mut Option<String> {
        self.part_tracking_data_mut().name_mut()
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

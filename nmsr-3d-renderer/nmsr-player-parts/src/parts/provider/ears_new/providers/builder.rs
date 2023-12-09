use glam::{Affine3A, Quat, Vec3};

use crate::{
    model::ArmorMaterial,
    parts::{
        part::Part,
        provider::{
            ears::providers::uv_utils::{TextureFlip, TextureRotation},
            minecraft::compute_base_part,
            PlayerPartProviderContext,
        },
        uv::uv_from_pos_and_size,
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
};

pub(crate) struct EarsModPartBuilder<'a, M: ArmorMaterial> {
    transformation_stack: Vec<Affine3A>,
    parts: &'a mut Vec<Part>,
    context: &'a PlayerPartProviderContext<M>,
}

impl<'a, M: ArmorMaterial> EarsModPartBuilder<'a, M> {
    pub(crate) fn new(
        target: &'a mut Vec<Part>,
        context: &'a PlayerPartProviderContext<M>,
    ) -> Self {
        Self {
            transformation_stack: vec![Affine3A::IDENTITY],
            parts: target,
            context,
        }
    }
    
    fn last_transformation(&self) -> &Affine3A {
        self.transformation_stack
            .last()
            .expect("Expected transformation stack to not be empty")
    }

    fn last_transformation_mut(&mut self) -> &mut Affine3A {
        self.transformation_stack
            .last_mut()
            .expect("Expected transformation stack to not be empty")
    }

    /// Compute the current transformation by multiplying all transformations in the stack
    pub(crate) fn current_transformation(&self) -> Affine3A {
        self.transformation_stack.iter().product()
    }

    pub(crate) fn push(&mut self) {
        self.transformation_stack.push(Affine3A::IDENTITY);
    }

    pub(crate) fn pop(&mut self) {
        self.transformation_stack
            .pop()
            .expect("Expected transformation stack to not be empty");
    }

    pub(crate) fn anchor_to(&mut self, part: PlayerBodyPartType) {
        let base_part =
            compute_base_part(part.get_non_layer_part(), self.context.model.is_slim_arms());

        let Vec3 { x, y, z } = base_part.get_position();

        self.translate(x, y, z);
    }

    pub(crate) fn stack<F: FnOnce(&mut Self) -> ()>(&mut self, action: F) {
        self.push();
        action(self);
        self.pop();
    }

    #[inline(always)]
    pub(crate) fn translate_i(&mut self, x: i32, y: i32, z: i32) {
        self.translate(x as f32, y as f32, z as f32);
    }
    
    pub(crate) fn translate(&mut self, x: f32, y: f32, z: f32) {
        let translation = Affine3A::from_translation([x, y, z].into());

        *self.last_transformation_mut() *= translation;
    }

    #[inline(always)]
    pub(crate) fn rotate_i(&mut self, value: i32, x: i32, y: i32, z: i32) {
        self.rotate((value * x) as f32, (value * y) as f32, (value * z) as f32);
    }
    
    pub(crate) fn rotate(&mut self, x: f32, y: f32, z: f32) {
        let rotation_quat = Quat::from_euler(
            glam::EulerRot::YXZ,
            y.to_radians(),
            x.to_radians(),
            z.to_radians(),
        );

        let rotation = Affine3A::from_quat(rotation_quat);

        *self.last_transformation_mut() *= Affine3A::from_quat(rotation_quat);
    }

    #[inline(always)]
    pub(crate) fn quad_double_sided(
        &mut self,
        u: u16,
        v: u16,
        width: u16,
        height: u16,
        rot: TextureRotation,
        flip: TextureFlip,
        name: impl Into<String>,
    ) {
        let name: String = name.into();
        
        self.quad_front(u, v, width, height, rot, flip, &name);
        self.quad_back(u, v, width, height, rot, flip.flip_horizontally(), name);
    }
    
    pub(crate) fn quad_front(
        &mut self,
        u: u16,
        v: u16,
        width: u16,
        height: u16,
        rot: TextureRotation,
        flip: TextureFlip,
        name: impl Into<String>,
    ) {
        let mut name: String = name.into();
        name.push_str(" (Front)");

        return self.quad(u, v, width, height, rot, flip, true, name);
    }

    pub(crate) fn quad_back(
        &mut self,
        u: u16,
        v: u16,
        width: u16,
        height: u16,
        rot: TextureRotation,
        flip: TextureFlip,
        name: impl Into<String>,
    ) {
        let mut name: String = name.into();
        name.push_str(" (Back)");

        return self.quad(u, v, width, height, rot, flip, false, name);
    }

    pub(crate) fn quad(
        &mut self,
        u: u16,
        v: u16,
        width: u16,
        height: u16,
        rot: TextureRotation,
        flip: TextureFlip,
        front_facing: bool,
        name: impl Into<String>,
    ) {
        return self.textured_quad(
            u,
            v,
            width,
            height,
            PlayerPartTextureType::Skin,
            rot,
            flip,
            front_facing,
            name,
        );
    }

    pub(crate) fn textured_quad(
        &mut self,
        u: u16,
        v: u16,
        mut width: u16,
        mut height: u16,
        texture: PlayerPartTextureType,
        rot: TextureRotation,
        mut flip: TextureFlip,
        front_facing: bool,
        name: impl Into<String>,
    ) {
        let z = if front_facing { 0.0 } else { 0.01 };

        let pos = [0.0, 0.0, z];
        let size = [width as u32, height as u32, 0];

        if rot.is_transposed() {
            std::mem::swap(&mut width, &mut height);

            flip = match flip {
                TextureFlip::Horizontal => TextureFlip::Vertical,
                TextureFlip::Vertical => TextureFlip::Horizontal,
                _ => flip,
            };
        }
        
        if front_facing {
            flip = flip.flip_horizontally();
        }
        
        let mut uvs = uv_from_pos_and_size(u, v, width, height);

        match rot {
            TextureRotation::Clockwise => uvs = uvs.rotate_cw(),
            TextureRotation::CounterClockwise => uvs = uvs.rotate_cw().rotate_cw().rotate_cw(),
            TextureRotation::UpsideDown => uvs = uvs.flip_vertically(),
            TextureRotation::None => {}
        }

        match flip {
            TextureFlip::Horizontal => uvs = uvs.flip_horizontally(),
            TextureFlip::Vertical => uvs = uvs.flip_vertically(),
            TextureFlip::Both => uvs = uvs.flip_horizontally().flip_vertically(),
            TextureFlip::None => {}
        }

        let mut quad = Part::new_quad(
            texture,
            pos,
            size,
            uvs,
            if front_facing { Vec3::NEG_Z } else { Vec3::Z },
            #[cfg(feature = "part_tracker")]
            Some(name.into()),
        );

        quad.transform_affine(self.current_transformation());

        self.parts.push(quad);
    }
}

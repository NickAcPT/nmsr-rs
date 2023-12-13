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
    mesh_stack: Vec<Vec<Part>>,
    texture_stack: Vec<PlayerPartTextureType>,
    group_stack: Vec<String>,
    parts: &'a mut Vec<Part>,
    context: &'a PlayerPartProviderContext<M>,
}

impl<'a, M: ArmorMaterial> EarsModPartBuilder<'a, M> {
    pub(crate) fn new(
        target: &'a mut Vec<Part>,
        context: &'a PlayerPartProviderContext<M>,
    ) -> Self {
        Self {
            texture_stack: vec![PlayerPartTextureType::Skin],
            transformation_stack: vec![Affine3A::IDENTITY],
            group_stack: vec![],
            mesh_stack: vec![],
            parts: target,
            context,
        }
    }

    fn current_texture(&self) -> PlayerPartTextureType {
        *self
            .texture_stack
            .last()
            .expect("Expected texture stack to not be empty")
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

    pub(crate) fn push_group(&mut self, name: impl Into<String>) {
        self.group_stack.push(name.into());
    }

    pub(crate) fn pop_group(&mut self) {
        self.group_stack
            .pop()
            .expect("Expected group stack to not be empty");
    }

    pub(crate) fn stack_group<F: FnOnce(&mut Self) -> ()>(
        &mut self,
        name: impl Into<String>,
        action: F,
    ) {
        self.push_group(name);
        action(self);
        self.pop_group();
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

    pub(crate) fn stack_texture<F: FnOnce(&mut Self) -> ()>(
        &mut self,
        texture: PlayerPartTextureType,
        action: F,
    ) {
        self.texture_stack.push(texture);
        self.stack(action);
        self.texture_stack.pop();
    }

    pub(crate) fn stack_mesh<F: FnOnce(&mut Self) -> ()>(
        &mut self,
        name: impl Into<String>,
        action: F,
    ) {
        let parts = vec![];
        self.mesh_stack.push(parts);

        self.stack(action);

        let parts = self
            .mesh_stack
            .pop()
            .expect("Expected mesh stack to not be empty");

        let mut part = Part::new_group(
            parts
                .first()
                .map(|p| p.get_texture())
                .unwrap_or(self.current_texture()),
            parts,
            #[cfg(feature = "part_tracker")]
            Some(name.into()),
        );

        part.transform_affine(self.current_transformation());

        #[cfg(feature = "part_tracker")]
        {
            part.push_groups(&self.group_stack);
        }

        self.parts.push(part);
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

    #[inline(always)]
    pub(crate) fn rotate_f(&mut self, value: f32, x: i32, y: i32, z: i32) {
        self.rotate(value * x as f32, value * y as f32, value * z as f32);
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

    pub(crate) fn scale(&mut self, x: f32, y: f32, z: f32) {
        let scale = Affine3A::from_scale([x, y, z].into());

        *self.last_transformation_mut() *= scale;
    }

    #[inline(always)]
    pub(crate) fn scale_i(&mut self, x: i32, y: i32, z: i32) {
        self.scale(x as f32, y as f32, z as f32);
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
        self.quad_double_sided_complete(
            u,
            v,
            u,
            v,
            width,
            height,
            rot,
            flip,
            rot,
            flip.flip_horizontally(),
            name,
        );
    }

    #[inline(always)]
    pub(crate) fn quad_double_sided_complete(
        &mut self,
        u_front: u16,
        v_front: u16,
        u_back: u16,
        v_back: u16,
        width: u16,
        height: u16,
        rot_front: TextureRotation,
        flip_front: TextureFlip,
        rot_back: TextureRotation,
        flip_back: TextureFlip,
        name: impl Into<String>,
    ) {
        let name: String = name.into();

        self.stack_group(name.clone(), move |b| {
            b.quad_front(
                u_front, v_front, width, height, rot_front, flip_front, &name,
            );
            b.quad_back(u_back, v_back, width, height, rot_back, flip_back, name);
        });
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
            self.current_texture(),
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
        let pos = [0.0, 0.0, 0.0];
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

        #[cfg(feature = "part_tracker")]
        {
            quad.push_groups(&self.group_stack);
        }

        if let (Part::Quad { normal, .. }, true) = (&quad, !front_facing) {
            quad.transform_affine(Affine3A::from_translation(*normal * 0.01));
        }

        if let Some(current_mesh) = self.mesh_stack.first_mut() {
            current_mesh.push(quad);
        } else {
            self.parts.push(quad);
        }
    }
}

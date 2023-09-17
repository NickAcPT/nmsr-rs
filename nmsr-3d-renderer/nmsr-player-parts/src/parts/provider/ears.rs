use std::{collections::HashMap, sync::OnceLock};

use ears_rs::features::EarsFeatures;
use itertools::Itertools;

use super::{PartsProvider, PlayerPartProviderContext};
use crate::{
    model::ArmorMaterial,
    parts::{part::{Part, PartAnchorInfo, self}, uv::{uv_from_pos_and_size, FaceUv}},
    types::{
        PlayerBodyPartType, PlayerBodyPartType::*, PlayerPartTextureType, PlayerPartTextureType::*,
    },
};

const ARM_PIXEL_CANARY: f32 = 0xe621 as f32;

macro_rules! declare_ears_parts {
    {$ears_part: ident {$($body:tt)+}} => {
        {
            EarsPlayerBodyPartDefinition {
                $($body)+,
                ..Default::default()
            }
        }
    };
}

#[derive(Debug, Copy, Clone)]
struct EarsPlayerBodyPartDefinition {
    texture: PlayerPartTextureType,
    pos: [f32; 3],
    rot: [f32; 3],
    size: [u32; 2],
    uv: [u16; 4],
    upside_down: bool,
    horizontal_flip: bool,
    enabled: fn(&EarsFeatures) -> bool,
}

impl Default for EarsPlayerBodyPartDefinition {
    fn default() -> Self {
        Self {
            texture: PlayerPartTextureType::Skin,
            pos: Default::default(),
            rot: Default::default(),
            size: Default::default(),
            uv: Default::default(),
            upside_down: Default::default(),
            horizontal_flip: Default::default(),
            enabled: |_| true,
        }
    }
}

pub(crate) struct EarsPlayerPartsProvider(
    HashMap<PlayerBodyPartType, Vec<EarsPlayerBodyPartDefinition>>,
);

impl Default for EarsPlayerPartsProvider {
    fn default() -> Self {
        let mut parts = Vec::new();

        parts.push((
            LeftLeg,
            vec![declare_ears_parts! {
                LeftLegClaw {
                    texture: Skin,
                    pos: [0.0, 0.0, -4.0],
                    size: [4, 4],
                    uv: [16, 48, 4, 4],
                    enabled: |f| f.claws
                }
            }],
        ));

        parts.push((
            RightLeg,
            vec![declare_ears_parts! {
                RightLegClaw {
                    texture: Skin,
                    pos: [0.0, 0.0, -4.0],
                    size: [4, 4],
                    uv: [0, 16, 4, 4],
                    enabled: |f| f.claws
                }
            }],
        ));

        parts.push((
            LeftArm,
            vec![declare_ears_parts! {
                LeftArmClaw {
                    pos: [-ARM_PIXEL_CANARY, 0.0, 0.0],
                    rot: [90.0, 0.0, 90.0],
                    size: [4, 4],
                    uv: [44, 48, 4, 4],
                    enabled: |f| f.claws,
                    upside_down: true
                }
            }],
        ));

        let mut map = HashMap::new();
        for (body_part, ears_parts) in parts {
            map.insert(body_part, ears_parts);
        }

        Self(map)
    }
}

#[inline(always)]
fn process_pos(pos: [f32; 3], is_slim_arms: bool) -> [f32; 3] {
    let mut pos = pos;

    for element in pos.as_mut_slice() {
        if (*element).abs() == ARM_PIXEL_CANARY {
            *element = if is_slim_arms { 3.0 } else { 4.0 } * ARM_PIXEL_CANARY.signum();
        }
    }

    pos
}

impl<M: ArmorMaterial> PartsProvider<M> for EarsPlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        let empty = Vec::with_capacity(0);

        if body_part.is_layer() || body_part.is_hat_layer() {
            return empty;
        }

        if let Some(features) = context.ears_features {
            let is_slim_arms = context.model.is_slim_arms();

            let mut result = Vec::new();

            if let Some(parts) = self.0.get(&body_part) {
                for part_definition in parts.iter().filter(|p| (p.enabled)(&features)) {
                    
                    let pos = process_pos(part_definition.pos, is_slim_arms);
                    let size = [part_definition.size[0], 0, part_definition.size[1]];
                    let uv = part_definition.uv;
                    
                    let mut uvs = FaceUv::from(uv_from_pos_and_size(uv[0], uv[1], uv[2], uv[3]));
                    
                    if part_definition.upside_down {
                        uvs = uvs.flip_vertically();
                    }
                    
                    if part_definition.horizontal_flip {
                        uvs = uvs.flip_horizontally();
                    }
                    
                    let mut part_quad = Part::new_quad(
                        part_definition.texture,
                        pos,
                        size,
                        uvs.into(),
                    );

                    part_quad.rotate(
                        part_definition.rot.into(),
                        Some(PartAnchorInfo::new_part_anchor_translate(
                            body_part,
                            is_slim_arms,
                        ).with_rotation_anchor(pos.into())),
                    );
                    
                    result.push(part_quad);
                    
                }
            }

            result
        } else {
            empty
        }
    }
}

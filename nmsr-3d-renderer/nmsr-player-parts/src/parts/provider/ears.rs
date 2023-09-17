use super::{PartsProvider, PlayerPartProviderContext};
use crate::{
    model::ArmorMaterial,
    parts::{
        part::{self, Part, PartAnchorInfo},
        uv::{uv_from_pos_and_size, FaceUv},
    },
    types::{PlayerBodyPartType, PlayerBodyPartType::*, PlayerPartTextureType},
};
use ears_rs::features::{data::wing::WingMode, EarsFeatures};
use glam::Vec3;
use std::collections::HashMap;

const ARM_PIXEL_CANARY: f32 = 0xe621 as f32;

macro_rules! declare_ears_part_horizontal {
    {$ears_part: ident {$($body:tt)+}} => {
        {
            EarsPlayerBodyPartDefinition {
                $($body)+,
                ..Default::default()
            }
        }
    };
}

macro_rules! declare_ears_part_vertical {
    {$ears_part: ident {$($body:tt)+}} => {
        {
            EarsPlayerBodyPartDefinition {
                $($body)+,
                vertical: true,
                ..Default::default()
            }
        }
    };
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerPartEarsTextureType {
    Cape,
    Wings,
    Emissive,
}

impl PlayerPartEarsTextureType {
    pub fn size(&self) -> (u32, u32) {
        match self {
            Self::Cape | Self::Wings => (20, 16),
            Self::Emissive => (64, 64),
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Cape => "ears_cape",
            Self::Wings => "ears_wings",
            Self::Emissive => "ears_emissive",
        }
    }
}

impl From<PlayerPartEarsTextureType> for PlayerPartTextureType {
    fn from(value: PlayerPartEarsTextureType) -> Self {
        match value {
            PlayerPartEarsTextureType::Cape => PlayerPartTextureType::Cape,
            ears => PlayerPartTextureType::Custom {
                key: ears.key(),
                size: ears.size(),
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct EarsPlayerBodyPartDefinition {
    texture: PlayerPartTextureType,
    pos: [f32; 3],
    rot: [f32; 3],
    size: [u32; 2],
    uv: [u16; 4],
    normal: Vec3,
    upside_down: bool,
    horizontal_flip: bool,
    enabled: fn(&EarsFeatures) -> bool,
    vertical: bool,
    double_sided: bool,
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
            normal: Vec3::Y,
            enabled: |_| true,
            vertical: false,
            double_sided: true,
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
            Head,
            vec![declare_ears_part_vertical! {
                Horn {
                    pos: [0.0, 8.0, 0.0],
                    rot: [-25.0, 0.0, 0.0],
                    size: [8, 8],
                    uv: [56, 0, 8, 8],
                    enabled: |f| f.horn,
                    normal: Vec3::Z
                }
            }],
        ));

        parts.push((
            LeftArm,
            vec![declare_ears_part_vertical! {
                LeftArmClaw {
                    pos: [0.0, -4.0, 4.0],
                    rot: [0.0, 90.0, 0.0],
                    size: [4, 4],
                    uv: [44, 48, 4, 4],
                    enabled: |f| f.claws,
                    upside_down: true,
                    horizontal_flip: true,
                    normal: Vec3::NEG_X
                }
            }],
        ));

        parts.push((
            RightArm,
            vec![declare_ears_part_vertical! {
                LeftArmClaw {
                    pos: [ARM_PIXEL_CANARY, -4.0, 4.0],
                    rot: [0.0, 90.0, 0.0],
                    size: [4, 4],
                    uv: [52, 16, 4, 4],
                    enabled: |f| f.claws,
                    normal: Vec3::X,
                    upside_down: true
                }
            }],
        ));

        parts.push((
            Body,
            vec![declare_ears_part_horizontal! {
                WingAsymmetricRight {
                    texture: PlayerPartEarsTextureType::Wings.into(),
                    pos: [8.0 - 2.0, 14.0, 4.0],
                    rot: [90.0, -60.0, 0.0],
                    size: [20, 16],
                    uv: [0, 0, 20, 16],
                    enabled: |f| f.wing.is_some_and(|w| w.mode == WingMode::AsymmetricR || w.mode == WingMode::SymmetricDual)
                }
            },
            declare_ears_part_horizontal! {
                WingAsymmetricLeft {
                    texture: PlayerPartEarsTextureType::Wings.into(),
                    pos: [2.0, 14.0, 4.0],
                    rot: [90.0, -120.0, 0.0],
                    size: [20, 16],
                    uv: [0, 0, 20, 16],
                    enabled: |f| f.wing.is_some_and(|w| w.mode == WingMode::AsymmetricL || w.mode == WingMode::SymmetricDual)
                }
            },
            declare_ears_part_horizontal! {
                WingAsymmetricSingle {
                    texture: PlayerPartEarsTextureType::Wings.into(),
                    pos: [4.0, 14.0, 4.0],
                    rot: [90.0, -90.0, 0.0],
                    size: [20, 16],
                    uv: [0, 0, 20, 16],
                    enabled: |f| f.wing.is_some_and(|w| w.mode == WingMode::SymmetricSingle)
                }
            }
            ],
        ));

        parts.push((
            LeftLeg,
            vec![declare_ears_part_horizontal! {
                LeftLegClaw {
                    pos: [0.0, 0.0, -4.0],
                    size: [4, 4],
                    uv: [16, 48, 4, 4],
                    enabled: |f| f.claws,
                    double_sided: false
                }
            }],
        ));

        parts.push((
            RightLeg,
            vec![declare_ears_part_horizontal! {
                RightLegClaw {
                    pos: [0.0, 0.0, -4.0],
                    size: [4, 4],
                    uv: [0, 16, 4, 4],
                    enabled: |f| f.claws,
                    double_sided: false
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

impl EarsPlayerPartsProvider {
    fn get_dynamic_parts(
        &self,
        body_part: PlayerBodyPartType,
        features: &EarsFeatures,
    ) -> Option<Vec<EarsPlayerBodyPartDefinition>> {
        match body_part {
            Head => Some(self.get_dynamic_head_parts(body_part, features)),
            _ => None,
        }
    }

    fn snout(body_part: PlayerBodyPartType, features: &EarsFeatures, result: &mut Vec<EarsPlayerBodyPartDefinition>) {
        
        if let Some(snout) = features
            .snout
            .filter(|s| s.width > 0 && s.height > 0 && s.depth > 0)
        {
            let snout_offset = snout.offset as f32;
            let snout_width = snout.width;
            let snout_height = snout.height;
            let snout_depth = snout.depth as f32;

            let snout_x = snout_width as f32 / 2.0;
            let snout_y = snout_offset;
            let snout_z = -snout_depth;

            macro_rules! snout_horizontal {
                ($y: expr, $normal: expr, $uv_y: expr, $uv_y_2: expr) => {
                    result.push(declare_ears_part_horizontal! {
                        SnoutFront {
                            pos: [snout_x, snout_y + $y as f32, snout_z],
                            size: [snout_width.into(), 1],
                            uv: [0, $uv_y, snout_width.into(), 1],
                            normal: $normal,
                            double_sided: false
                        }
                    });

                    result.push(declare_ears_part_horizontal! {
                        SnoutRest {
                            pos: [snout_x, snout_y + $y as f32, snout_z as f32 + 1.0],
                            size: [snout_width.into(), snout_depth as u32],
                            uv: [0, $uv_y_2, snout_width.into(), 1],
                            normal: $normal,
                            double_sided: false
                        }
                    });
                };
            }
            
            macro_rules! snout_vertical {
                ($x: expr, $normal: expr, $uv_y_1: expr, $uv_y_2: expr) => {
                    result.push(declare_ears_part_vertical! {
                        SnoutSideFront {
                            pos: [snout_x + $x as f32, snout_y, snout_z + 1.0],
                            rot: [0.0, 90.0, 0.0],
                            size: [1, snout_height.into()],
                            uv: [7, $uv_y_1, 1, snout_height.into()],
                            normal: $normal,
                            double_sided: false
                        }
                    });
                    
                    result.push(declare_ears_part_vertical! {
                        SnoutSideRest {
                            pos: [snout_x + $x as f32, snout_y, 0.0],
                            rot: [0.0, 90.0, 0.0],
                            size: [snout_depth as u32 - 1, snout_height.into()],
                            uv: [7, $uv_y_2, 1, snout_height.into()],
                            normal: $normal,
                            double_sided: false
                        }
                    });
                };
            }
            
            result.push(declare_ears_part_vertical! {
                SnoutFront {
                    pos: [snout_x, snout_y, snout_z],
                    size: [snout_width.into(), snout_height.into()],
                    uv: [0, 2, snout_width.into(), snout_height.into()],
                    normal: Vec3::NEG_Z,
                    double_sided: false
                }
            });
            
            snout_horizontal!(snout_height, Vec3::Y, 1, 0);
            snout_horizontal!(0.0, Vec3::NEG_Y, 2 + snout_height as u16, 3 + snout_height as u16);
            
            snout_vertical!(snout_width, Vec3::X, 0, 4);
            snout_vertical!(0.0, Vec3::NEG_X, 0, 4);
        }
    }
    
    fn get_dynamic_head_parts(
        &self,
        body_part: PlayerBodyPartType,
        features: &EarsFeatures,
    ) -> Vec<EarsPlayerBodyPartDefinition> {
        let mut result = Vec::new();

        Self::snout(body_part, features, &mut result);
        
        result
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
                let dynamic_parts = self.get_dynamic_parts(body_part, &features);

                let processed_parts = parts
                    .iter()
                    .chain(dynamic_parts.iter().flatten())
                    .filter(|p| (p.enabled)(&features))
                    .flat_map(|&p| {
                        if p.double_sided {
                            let mut back = p.clone();
                            back.normal *= -1.0;
                            back.horizontal_flip ^= true;

                            vec![p, back]
                        } else {
                            vec![p]
                        }
                    });

                for part_definition in processed_parts {
                    let pos = process_pos(part_definition.pos, is_slim_arms);
                    let size = if part_definition.vertical {
                        [part_definition.size[0], part_definition.size[1], 0]
                    } else {
                        [part_definition.size[0], 0, part_definition.size[1]]
                    };

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
                        part_definition.normal,
                    );

                    part_quad.rotate(
                        part_definition.rot.into(),
                        Some(
                            PartAnchorInfo::new_part_anchor_translate(body_part, is_slim_arms)
                                .with_rotation_anchor(pos.into()),
                        ),
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

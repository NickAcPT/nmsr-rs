use super::{PartsProvider, PlayerPartProviderContext};
use crate::{
    model::ArmorMaterial,
    parts::{
        part::{Part, PartAnchorInfo},
        uv::{uv_from_pos_and_size, FaceUv},
    },
    types::{PlayerBodyPartType, PlayerBodyPartType::*, PlayerPartTextureType},
};

use ears_rs::features::{
    data::{
        ear::{EarAnchor, EarMode},
        tail::{self, TailMode},
        wing::WingMode,
    },
    EarsFeatures,
};
use glam::Vec3;
use itertools::Itertools;
use std::collections::HashMap;

const ARM_PIXEL_CANARY: f32 = 0xe621 as f32;
const PREV_CORNER_CANARY: f32 = 0xe926 as f32;

macro_rules! declare_ears_part_horizontal {
    {$ears_part: ident {$($body:tt)+}} => {
        {
            EarsPlayerBodyPartDefinition {
                $($body)+,
                name: stringify!($ears_part),
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
                vertical_quad: true,
                name: stringify!($ears_part),
                ..Default::default()
            }
        }
    };
}

macro_rules! rot {
    {$({$($body:tt)+} $(,)*)* $(,)*} => {
        {
            vec![$(
                EarsPlayerBodyPartRotation {
                    $($body)+,
                    ..Default::default()
                },
            )+]
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

#[derive(Debug, Clone, Copy, Default)]
struct EarsPlayerBodyPartRotation {
    rot: [f32; 3],
    rot_anchor: [f32; 3],
}

#[derive(Debug, Clone)]
struct EarsPlayerBodyPartDefinition {
    texture: PlayerPartTextureType,
    pos: [f32; 3],
    rot_stack: Vec<EarsPlayerBodyPartRotation>,
    size: [u16; 2],
    uv: [u16; 4],
    back_uv: Option<[u16; 4]>,
    normal: Vec3,
    vertical_flip: bool,
    horizontal_flip: bool,
    cw: bool,
    back_cw: Option<bool>,
    enabled: fn(&EarsFeatures) -> bool,
    vertical_quad: bool,
    double_sided: bool,
    is_back: bool,
    name: &'static str,
    part_count: Option<u32>,
    reset_rotation_stack: bool,
}

impl Default for EarsPlayerBodyPartDefinition {
    fn default() -> Self {
        Self {
            texture: PlayerPartTextureType::Skin,
            pos: Default::default(),
            rot_stack: Default::default(),
            size: Default::default(),
            uv: Default::default(),
            back_uv: None,
            vertical_flip: Default::default(),
            horizontal_flip: Default::default(),
            cw: false,
            back_cw: None,
            normal: Vec3::Y,
            enabled: |_| true,
            vertical_quad: false,
            double_sided: true,
            name: "",
            part_count: None,
            is_back: false,
            reset_rotation_stack: false,
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
                    rot_stack: rot! {
                        { rot: [-25.0, 0.0, 0.0] }
                    },
                    size: [8, 8],
                    uv: [56, 0, 8, 8],
                    enabled: |f| f.horn,
                    double_sided: false,
                    normal: Vec3::NEG_Z
                }
            }],
        ));

        parts.push((
            LeftArm,
            vec![declare_ears_part_vertical! {
                LeftArmClaw {
                    pos: [0.0, -4.0, 4.0],
                    rot_stack: rot!{
                        { rot: [0.0, 90.0, 0.0] }
                    },
                    size: [4, 4],
                    uv: [44, 48, 4, 4],
                    enabled: |f| f.claws,
                    vertical_flip: true,
                    double_sided: false,
                    normal: Vec3::NEG_X
                }
            }],
        ));

        parts.push((
            RightArm,
            vec![declare_ears_part_vertical! {
                RightArmClaw {
                    pos: [ARM_PIXEL_CANARY, -4.0, 4.0],
                    rot_stack: rot! {
                        { rot: [0.0, 90.0, 0.0] }
                    },
                    size: [4, 4],
                    uv: [52, 16, 4, 4],
                    enabled: |f| f.claws,
                    normal: Vec3::X,
                    vertical_flip: true,
                    double_sided: false
                }
            }],
        ));

        parts.push((
            Body,
            vec![declare_ears_part_vertical! {
                WingAsymmetricRight {
                    texture: PlayerPartEarsTextureType::Wings.into(),
                    pos: [8.0 - 2.0, -2.0, 4.0],
                    rot_stack: rot!{
                        { rot: [0.0, -60.0, 0.0] }
                    },
                    size: [20, 16],
                    uv: [0, 0, 20, 16],
                    normal: Vec3::X,
                    horizontal_flip: true,
                    enabled: |f| f.wing.is_some_and(|w| w.mode == WingMode::AsymmetricR || w.mode == WingMode::SymmetricDual)
                }
            },
            declare_ears_part_vertical! {
                WingAsymmetricLeft {
                    texture: PlayerPartEarsTextureType::Wings.into(),
                    pos: [2.0, -2.0, 4.0],
                    rot_stack: rot!{
                        { rot: [0.0, -120.0, 0.0] }
                    },
                    size: [20, 16],
                    uv: [0, 0, 20, 16],
                    normal: Vec3::NEG_X,
                    enabled: |f| f.wing.is_some_and(|w| w.mode == WingMode::AsymmetricL || w.mode == WingMode::SymmetricDual)
                }
            },
            declare_ears_part_vertical! {
                WingSymmetricSingle {
                    texture: PlayerPartEarsTextureType::Wings.into(),
                    pos: [4.0, -2.0, 4.0],
                    rot_stack: rot!{
                        { rot: [0.0, -90.0, 0.0] }
                    },
                    size: [20, 16],
                    uv: [0, 0, 20, 16],
                    normal: Vec3::NEG_X,
                    horizontal_flip: true,
                    double_sided: false,
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
            Body => Some(self.tails(body_part, features)), //TODO: Chest
            _ => None,
        }
    }

    #[allow(clippy::needless_update)]
    fn tails(
        &self,
        body_part: PlayerBodyPartType,
        features: &EarsFeatures,
    ) -> Vec<EarsPlayerBodyPartDefinition> {
        let mut result = Vec::new();

        let Some(tail_data) = features.tail else {
            return result;
        };

        let vertical = tail_data.mode == TailMode::Vertical;

        let bend_0 = tail_data.bends[0];

        let angle = match tail_data.mode {
            tail::TailMode::Down => 30.0,
            tail::TailMode::Back => {
                if bend_0 != 0.0 {
                    90.0
                } else {
                    80.0
                }
            }
            tail::TailMode::Up => 130.0,
            _ => 0.0,
        };

        let z_rotation = if vertical { 90.0 } else { 0.0 };

        let mut bends = tail_data.bends;
        
        if vertical {
            bends[0] = 0.0;
        }

        let vertical_rotation = if vertical {
            90.0
        } else {
            0.0
        };
        
        let segments = tail_data.segments.clamp(1, 4) as usize;
        let seg_height = 12.0 / segments as f32;
        let seg_height_u16 = seg_height as u16;

        let mut rot_x_acc = angle;
        for segment in 0..segments {
            rot_x_acc += tail_data.bends[segment];

            result.push(declare_ears_part_vertical!(TailSegment {
                pos: [
                    8.0,
                    2.0 + PREV_CORNER_CANARY + (seg_height * segment as f32),
                    4.0 + PREV_CORNER_CANARY
                ],
                rot_stack: rot! {
                    {
                        rot: [rot_x_acc - 180.0, 180.0, 0.0]
                    },
                    {
                        rot: [0.0, vertical_rotation, -vertical_rotation],
                        rot_anchor: [-4.0, 0.0, 0.0]
                    }
                },
                size: [8, seg_height_u16],
                uv: [
                    56,
                    16 + (segment as u16 * seg_height_u16),
                    8,
                    seg_height_u16
                ],
                normal: Vec3::Z,
                part_count: Some(segment as u32),
                vertical_flip: true,
                horizontal_flip: vertical,
                reset_rotation_stack: segment == 0,
                double_sided: false
            }));
        }

        result
    }

    fn ears(
        body_part: PlayerBodyPartType,
        features: &EarsFeatures,
        result: &mut Vec<EarsPlayerBodyPartDefinition>,
    ) {
        let mut anchor = features.ear_anchor.unwrap_or_default();
        let mut mode = features.ear_mode;

        // Upgrade the old ear mode to the new one
        if mode == EarMode::Behind {
            mode = EarMode::Around;
            anchor = EarAnchor::Back;
        }

        let anchor_z = match anchor {
            EarAnchor::Front => 0.0,
            EarAnchor::Center => 4.0,
            EarAnchor::Back => 8.0,
        };

        if mode == EarMode::Above || mode == EarMode::Around {
            result.push(declare_ears_part_vertical! {
                EarMiddle {
                    pos: [-4.0, 8.0, anchor_z],
                    size: [16, 8],
                    uv: [24, 0, 16, 8],
                    back_uv: Some([56, 28, 16, 8]),
                    back_cw: Some(true),
                    normal: Vec3::NEG_Z
                }
            });

            if mode == EarMode::Around {
                result.push(declare_ears_part_vertical! {
                    EarAroundRight {
                        pos: [8.0, 0.0, anchor_z],
                        size: [4, 8],
                        uv: [36, 16, 4, 8],
                        back_uv: Some([12, 16, 4, 8]),
                        normal: Vec3::NEG_Z,
                        cw: true
                    }
                });

                result.push(declare_ears_part_vertical! {
                    EarAroundLeft {
                        pos: [-4.0, 0.0, anchor_z],
                        size: [4, 8],
                        uv: [36, 32, 4, 8],
                        back_uv: Some([12, 32, 4, 8]),
                        normal: Vec3::NEG_Z,
                        cw: true
                    }
                });
            }
        } else if mode == EarMode::Sides {
            result.push(declare_ears_part_vertical! {
                EarSidesLeft {
                    pos: [-8.0, 0.0, anchor_z],
                    size: [8, 8],
                    uv: [32, 0, 8, 8],
                    back_uv: Some([56, 36, 8, 8]),
                    normal: Vec3::NEG_Z,
                    back_cw: Some(true)
                }
            });
            result.push(declare_ears_part_vertical! {
                EarSidesRight {
                    pos: [8.0, 0.0, anchor_z],
                    size: [8, 8],
                    uv: [24, 0, 8, 8],
                    back_uv: Some([56, 28, 8, 8]),
                    normal: Vec3::NEG_Z,
                    back_cw: Some(true)
                }
            });
        } else if mode == EarMode::Floppy {
            result.push(declare_ears_part_vertical! {
                EarFloppyRight {
                    pos: [8.0, 0.0, 0.0],
                    size: [8, 8],
                    rot_stack: rot!{
                        {
                            rot: [30.0, -90.0, 0.0],
                            rot_anchor: [0.0, 7.0, 0.0]
                        }
                    },
                    uv: [24, 0, 8, 8],
                    back_uv: Some([56, 28, 8, 8]),
                    normal: Vec3::X,
                    back_cw: Some(true)
                }
            });

            result.push(declare_ears_part_vertical! {
                EarFloppyLeft {
                    pos: [0.0, 0.0, 8.0],
                    size: [8, 8],
                    rot_stack: rot!{
                        {
                            rot: [30.0, 90.0, 0.0],
                            rot_anchor: [0.0, 7.0, 0.0]
                        }
                    },
                    uv: [32, 0, 8, 8],
                    back_uv: Some([56, 36, 8, 8]),
                    normal: Vec3::NEG_X,
                    back_cw: Some(true)
                }
            });
        } else if mode == EarMode::Out {
            let (pos_y, pos_z) = match anchor {
                EarAnchor::Center => (8.0, 0.0),
                EarAnchor::Front => (0.0, -8.0),
                EarAnchor::Back => (0.0, 8.0),
            };

            result.push(declare_ears_part_vertical! {
                EarOutRight {
                    pos: [8.0, pos_y, pos_z],
                    size: [8, 8],
                    rot_stack: rot!{
                        { rot: [0.0, -90.0, 0.0] }
                    },
                    uv: [24, 0, 8, 8],
                    back_uv: Some([56, 28, 8, 8]),
                    normal: Vec3::X,
                    back_cw: Some(true)
                }
            });

            result.push(declare_ears_part_vertical! {
                EarOutLeft {
                    pos: [0.0, pos_y, 8.0 + pos_z],
                    size: [8, 8],
                    rot_stack: rot!{
                        { rot: [0.0, 90.0, 0.0] }
                    },
                    uv: [32, 0, 8, 8],
                    back_uv: Some([56, 36, 8, 8]),
                    normal: Vec3::NEG_X,
                    back_cw: Some(true)
                }
            });
        } else if mode == EarMode::Tall {
            let angle = 6.0;

            let mut current_angle = angle / 3.0;

            result.push(declare_ears_part_vertical! {
                EarTallOne {
                    pos: [0.0, 8.0, anchor_z],
                    rot_stack: rot! {
                        { rot: [current_angle, 0.0, 0.0] }
                    },
                    size: [8, 4],
                    uv: [24, 0, 8, 4],
                    back_uv: Some([56, 40, 8, 4]),
                    normal: Vec3::NEG_Z,
                    cw: true,
                    back_cw: Some(false),
                    reset_rotation_stack: true
                }
            });

            current_angle += angle;

            result.push(declare_ears_part_vertical! {
                EarTallTwo {
                    pos: [0.0, 8.0 + 4.0 + PREV_CORNER_CANARY, anchor_z + PREV_CORNER_CANARY],
                    size: [8, 4],
                    rot_stack: rot! {
                        { rot: [current_angle, 0.0, 0.0] }
                    },
                    uv: [28, 0, 8, 4],
                    back_uv: Some([56, 36, 8, 4]),
                    normal: Vec3::NEG_Z,
                    cw: true,
                    back_cw: Some(false)
                }
            });

            current_angle += angle / 2.0;

            result.push(declare_ears_part_vertical! {
                EarTallThree {
                    pos: [0.0, 8.0 + 8.0 + PREV_CORNER_CANARY, anchor_z + PREV_CORNER_CANARY],
                    size: [8, 4],
                    rot_stack: rot! {
                        { rot: [current_angle, 0.0, 0.0] }
                    },
                    uv: [32, 0, 8, 4],
                    back_uv: Some([56, 32, 8, 4]),
                    normal: Vec3::NEG_Z,
                    cw: true,
                    back_cw: Some(false)
                }
            });

            current_angle += angle;

            result.push(declare_ears_part_vertical! {
                EarTallFour {
                    pos: [0.0, 8.0 + 12.0 + PREV_CORNER_CANARY, anchor_z + PREV_CORNER_CANARY],
                    size: [8, 4],
                    rot_stack: rot! {
                        { rot: [current_angle, 0.0, 0.0] }
                    },
                    uv: [36, 0, 8, 4],
                    back_uv: Some([56, 28, 8, 4]),
                    normal: Vec3::NEG_Z,
                    cw: true,
                    back_cw: Some(false)
                }
            });
        } else if mode == EarMode::TallCross {
            result.push(declare_ears_part_vertical! {
                EarTallLeft {
                    pos: [1.0, 8.0, anchor_z - 3.0],
                    size: [8, 16],
                    rot_stack: rot! {
                        { rot: [0.0, -45.0, 0.0] }
                    },
                    uv: [24, 0, 8, 16],
                    back_uv: Some([56, 28, 8, 16]),
                    normal: Vec3::NEG_X,
                    cw: true,
                    back_cw: Some(false)
                }
            });
            result.push(declare_ears_part_vertical! {
                EarTallRight {
                    pos: [1.0, 8.0, anchor_z + 3.0],
                    size: [8, 16],
                    rot_stack: rot! {
                        { rot: [0.0, 45.0, 0.0] }
                    },
                    uv: [24, 0, 8, 16],
                    back_uv: Some([56, 28, 8, 16]),
                    normal: Vec3::X,
                    cw: true,
                    back_cw: Some(false)
                }
            });
        }
    }

    fn snout(
        body_part: PlayerBodyPartType,
        features: &EarsFeatures,
        result: &mut Vec<EarsPlayerBodyPartDefinition>,
    ) {
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
                ($name: ident, $name_2: ident, $y: expr, $normal: expr, $uv_y: expr, $uv_y_2: expr) => {
                    result.push(declare_ears_part_horizontal! {
                        $name {
                            pos: [snout_x, snout_y + $y as f32, snout_z],
                            size: [snout_width.into(), 1],
                            uv: [0, $uv_y, snout_width.into(), 1],
                            normal: $normal,
                            double_sided: false
                        }
                    });

                    result.push(declare_ears_part_horizontal! {
                        $name_2 {
                            pos: [snout_x, snout_y + $y as f32, snout_z as f32 + 1.0],
                            size: [snout_width.into(), snout_depth as u16],
                            uv: [0, $uv_y_2, snout_width.into(), 1],
                            normal: $normal,
                            double_sided: false
                        }
                    });
                };
            }

            macro_rules! snout_vertical {
                ($name: ident, $name_2: ident, $x: expr, $normal: expr, $uv_y_1: expr, $uv_y_2: expr) => {
                    result.push(declare_ears_part_vertical! {
                        $name {
                            pos: [snout_x + $x as f32, snout_y, snout_z + 1.0],
                            rot_stack: rot! {
                                { rot: [0.0, 90.0, 0.0] }
                            },
                            size: [1, snout_height.into()],
                            uv: [7, $uv_y_1, 1, snout_height.into()],
                            normal: $normal,
                            double_sided: false
                        }
                    });

                    result.push(declare_ears_part_vertical! {
                        $name_2 {
                            pos: [snout_x + $x as f32, snout_y, 0.0],
                            rot_stack: rot! {
                                { rot: [0.0, 90.0, 0.0] }
                            },
                            size: [snout_depth as u16 - 1, snout_height.into()],
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

            snout_horizontal!(SnoutTopFront, SnoutTopRest, snout_height, Vec3::Y, 1, 0);
            snout_horizontal!(
                SnoutBottomFront,
                SnoutBottomRest,
                0.0,
                Vec3::NEG_Y,
                2 + snout_height as u16,
                3 + snout_height as u16
            );

            snout_vertical!(SnoutRightFront, SnoutRightRest, snout_width, Vec3::X, 0, 4);
            snout_vertical!(SnoutLeftFront, SnoutLeftRest, 0.0, Vec3::NEG_X, 0, 4);
        }
    }

    fn get_dynamic_head_parts(
        &self,
        body_part: PlayerBodyPartType,
        features: &EarsFeatures,
    ) -> Vec<EarsPlayerBodyPartDefinition> {
        let mut result = Vec::new();

        Self::snout(body_part, features, &mut result);
        Self::ears(body_part, features, &mut result);

        result
    }
}

#[inline(never)]
fn process_pos(pos: [f32; 3], is_slim_arms: bool, last_pos: &[f32; 3]) -> [f32; 3] {
    let mut pos = pos;

    for element in pos.as_mut_slice() {
        if (*element).abs() == ARM_PIXEL_CANARY {
            *element = if is_slim_arms { 3.0 } else { 4.0 } * element.signum();
        }
    }

    for (index, element) in pos.iter_mut().enumerate() {
        if (*element).abs() == PREV_CORNER_CANARY {
            *element = last_pos[index] * element.signum();
        } else if (*element) > PREV_CORNER_CANARY {
            let rest = *element - PREV_CORNER_CANARY;
            *element = last_pos[index] + rest;
        } else if (*element) < -PREV_CORNER_CANARY {
            let rest = *element + PREV_CORNER_CANARY;
            *element = last_pos[index] + rest;
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

            if let Some(parts) = self.0.get(&body_part).cloned() {
                let dynamic_parts = self
                    .get_dynamic_parts(body_part, &features)
                    .unwrap_or(Vec::with_capacity(0));

                let processed_parts = parts
                    .into_iter()
                    .chain(dynamic_parts)
                    .filter(|p| (p.enabled)(&features))
                    .flat_map(|p| {
                        if p.double_sided {
                            let mut back = p.clone();
                            back.normal *= -1.0;

                            back.uv = p.back_uv.unwrap_or(p.uv);
                            back.cw = p.back_cw.unwrap_or(back.cw);

                            if back.cw {
                                back.vertical_flip ^= true;
                            } else {
                                back.horizontal_flip ^= true;
                            }

                            back.is_back = true;

                            vec![p, back]
                        } else {
                            vec![p]
                        }
                    })
                    .sorted_by_key(|p| p.is_back);

                let mut last_pos = Vec3::ZERO;
                for part_definition in processed_parts {
                    if part_definition.reset_rotation_stack {
                        last_pos = Vec3::ZERO;
                        println!();
                        println!("## Reset rotation stack ##");
                        println!();
                    }

                    let size = part_definition.size;

                    let uvs = process_uvs(
                        part_definition.uv,
                        part_definition.horizontal_flip,
                        part_definition.vertical_flip,
                        part_definition.cw,
                        part_definition.vertical_quad,
                    );

                    let pos = process_pos(part_definition.pos, is_slim_arms, &last_pos.into());

                    let size = if part_definition.vertical_quad {
                        [size[0] as u32, size[1] as u32, 0]
                    } else {
                        [size[0] as u32, 0, size[1] as u32]
                    };

                    let normal_offset = if part_definition.double_sided {
                        part_definition.normal * 0.01
                    } else {
                        Vec3::ZERO
                    };

                    #[cfg(feature = "part_tracker")]
                    let mut name = String::from(part_definition.name);

                    #[cfg(feature = "part_tracker")]
                    {
                        if let Some(count) = part_definition.part_count {
                            name.push_str(&count.to_string());
                        }

                        if part_definition.is_back {
                            name.push_str("Back");
                        }
                    }

                    let mut part_quad = Part::new_quad(
                        part_definition.texture,
                        [0.0; 3],
                        size,
                        uvs,
                        part_definition.normal,
                        #[cfg(feature = "part_tracker")]
                        Some(name.clone()),
                    );

                    println!();
                    println!(" #### Doing {name} part rotation ####");
                    println!();

                    for (index, EarsPlayerBodyPartRotation { rot, rot_anchor }) in
                        part_definition.rot_stack.into_iter().enumerate()
                    {
                        let anchor = if index == 0 {
                            PartAnchorInfo::new_part_anchor_translate(body_part, is_slim_arms)
                                .with_rotation_anchor(Vec3::from(pos))
                        } else {
                            PartAnchorInfo::new_part_anchor_translate(body_part, is_slim_arms)
                                .with_rotation_anchor(Vec3::from(pos))
                                .without_translation_anchor()
                        };

                        let rot_anchor = process_pos(rot_anchor, is_slim_arms, &last_pos.into());

                        part_quad.rotate(
                            rot.into(),
                            Some(anchor.with_rotation_anchor(Vec3::from(rot_anchor))),
                        );
                    }

                    part_quad.translate(Vec3::from(pos));

                    let new_offset = part_quad
                        .get_rotation_matrix()
                        .transform_vector3(normal_offset);

                    *part_quad.position_mut() += new_offset;

                    let pos = part_quad.get_position();
                    let size = part_quad.get_size();

                    let old_point = if part_definition.vertical_quad {
                        [pos[0] + size[0], pos[1] + size[1] as f32, pos[2]]
                    } else {
                        [pos[0], pos[1], pos[2] - size[1] as f32]
                    };
                    
                    let old = Vec3::from(dbg!(old_point)) - normal_offset;

                    dbg!(part_quad.get_rotation_matrix().transform_point3(old_point.into()));
                    
                    last_pos += (part_quad.get_rotation_matrix().transform_point3(old)) - (old);

                    result.push(part_quad);
                }
            }

            result
        } else {
            empty
        }
    }
}

fn process_uvs(
    mut uv: [u16; 4],
    horizontal_flip: bool,
    upside_down: bool,
    cw: bool,
    vertical: bool,
) -> FaceUv {
    if cw {
        uv.swap(2, 3);
    }

    let mut uvs = uv_from_pos_and_size(uv[0], uv[1], uv[2], uv[3]);

    if upside_down {
        uvs = uvs.flip_vertically();
    }

    if horizontal_flip {
        uvs = uvs.flip_horizontally();
    }

    if cw {
        uvs = uvs.rotate_cw();
    }

    uvs = uvs.flip_horizontally();

    uvs
}

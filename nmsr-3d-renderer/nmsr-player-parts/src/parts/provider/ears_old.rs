use super::{PartsProvider, PlayerPartProviderContext};
#[cfg(feature = "markers")]
use crate::parts::part::Marker;
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum PositionCanary {
    None(f32),
    ArmPixelCanary,
    PrevCornerCanary(f32),
}

impl std::ops::Add<PositionCanary> for f32 {
    type Output = PositionCanary;

    fn add(self, rhs: PositionCanary) -> Self::Output {
        match rhs {
            PositionCanary::None(value) => PositionCanary::None(self + value),
            PositionCanary::ArmPixelCanary => unimplemented!("Arm pixel canary is not supported"),
            PositionCanary::PrevCornerCanary(value) => {
                PositionCanary::PrevCornerCanary(self + value)
            }
        }
    }
}

impl std::ops::Add<f32> for PositionCanary {
    type Output = PositionCanary;

    fn add(self, rhs: f32) -> Self::Output {
        rhs + self
    }
}

impl Default for PositionCanary {
    fn default() -> Self {
        Self::None(0.0)
    }
}

impl PositionCanary {
    fn resolve(&self, index: usize, is_slim_arms: bool, last_pos: &[f32; 3]) -> f32 {
        match self {
            Self::None(value) => *value,
            Self::ArmPixelCanary => {
                if is_slim_arms {
                    3.0
                } else {
                    4.0
                }
            }
            Self::PrevCornerCanary(value) => last_pos[index] + value,
        }
    }
}

impl From<f32> for PositionCanary {
    fn from(value: f32) -> Self {
        Self::None(value)
    }
}

const ARM_PIXEL_CANARY: PositionCanary = PositionCanary::ArmPixelCanary;
const PREV_CORNER_CANARY: PositionCanary = PositionCanary::PrevCornerCanary(0.0);

macro_rules! pos {
    ($value: expr) => {
        $value.into()
    };
    [$( $value: expr ),*] => {
        [$(
            pos!($value)
        ),*]
    };
}

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

#[derive(Debug, Clone)]
struct EarsPlayerBodyPartDefinition {
    texture: PlayerPartTextureType,
    pos: [PositionCanary; 3],
    rot: [f32; 3],
    rot_anchor: [PositionCanary; 3],
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
            rot: Default::default(),
            rot_anchor: Default::default(),
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
                    pos: pos![0.0, 8.0, 0.0],
                    rot: [-25.0, 0.0, 0.0],
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
                    pos: pos![0.0, -4.0, 4.0],
                    rot: [0.0, 90.0, 0.0],
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
                    pos: pos![ARM_PIXEL_CANARY, -4.0, 4.0],
                    rot: [0.0, 90.0, 0.0],
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
                    pos: pos![8.0 - 2.0, -2.0, 4.0],
                    rot: [0.0, -60.0, 0.0],
                    size: [20, 16],
                    uv: [0, 0, 20, 16],
                    normal: Vec3::X,
                    horizontal_flip: true,
                    double_sided: false,
                    enabled: |f| f.wing.is_some_and(|w| w.mode == WingMode::AsymmetricR || w.mode == WingMode::SymmetricDual)
                }
            },
            declare_ears_part_vertical! {
                WingAsymmetricLeft {
                    texture: PlayerPartEarsTextureType::Wings.into(),
                    pos: pos![2.0, -2.0, 4.0],
                    rot: [0.0, -120.0, 0.0],
                    size: [20, 16],
                    uv: [0, 0, 20, 16],
                    normal: Vec3::NEG_X,
                    horizontal_flip: true,
                    double_sided: false,
                    enabled: |f| f.wing.is_some_and(|w| w.mode == WingMode::AsymmetricL || w.mode == WingMode::SymmetricDual)
                }
            },
            declare_ears_part_vertical! {
                WingSymmetricSingle {
                    texture: PlayerPartEarsTextureType::Wings.into(),
                    pos: pos![4.0, -2.0, 4.0],
                    rot: [0.0, -90.0, 0.0],
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
                    pos: pos![0.0, 0.0, -4.0],
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
                    pos: pos![0.0, 0.0, -4.0],
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
            // TODO: Fix this one of these days
            bends[0] = 0.0;
        }

        let vertical_rotation = if vertical { 90.0 } else { 0.0 };

        let segments = tail_data.segments.clamp(1, 4) as usize;
        let seg_height = 12.0 / segments as f32;
        let seg_height_u16 = seg_height as u16;

        let mut rot_x_acc = angle;
        for segment in 0..segments {
            rot_x_acc += bends[segment];

            let tail_pos = if vertical {
                [4.0, 4.0, 4.0]
            } else {
                [8.0, 2.0 + (seg_height * segment as f32), 4.0]
            };

            let tail_x = if segment == 0 {
                tail_pos[0].into()
            } else {
                PREV_CORNER_CANARY + if vertical { 0.0 } else { 8.0 }
            };
            let tail_y = if segment == 0 {
                tail_pos[1].into()
            } else {
                PREV_CORNER_CANARY + if vertical { 8.0 } else { 0.0 }
            };

            let tail_z = if segment == 0 {
                tail_pos[2].into()
            } else {
                PREV_CORNER_CANARY
            };

            let pos = [tail_x, tail_y, tail_z];

            let rot = if vertical {
                [0.0, 90.0 - rot_x_acc, -180.0]
            } else {
                [rot_x_acc - 180.0, 180.0, 0.0]
            };

            let mut size = [8, seg_height_u16];
            let mut uv = [
                56,
                16 + (segment as u16 * seg_height_u16),
                8,
                seg_height_u16,
            ];

            if vertical {
                size.swap(0, 1);
                uv.swap(2, 3);
            }

            let tail = declare_ears_part_vertical!(TailSegment {
                pos,
                rot,
                size,
                uv,
                normal: Vec3::Z,
                part_count: Some(segment as u32),
                vertical_flip: true,
                horizontal_flip: vertical,
                cw: vertical,
                reset_rotation_stack: segment == 0,
                double_sided: false
            });

            result.push(tail);
        }

        result
    }

    fn ears(
        body_part: PlayerBodyPartType,
        features: &EarsFeatures,
        result: &mut Vec<EarsPlayerBodyPartDefinition>,
    ) {
        let mut anchor = features.ear_anchor;
        let mut mode = features.ear_mode;

        // Upgrade the old ear mode to the new one
        if mode == EarMode::Behind {
            mode = EarMode::Out;
            anchor = EarAnchor::Back;
        }

        let anchor_z = match anchor {
            EarAnchor::Front => 0.0,
            EarAnchor::Center => 4.0,
            EarAnchor::Back => 8.0,
        };

        match mode {
            EarMode::Above | EarMode::Around => {
                result.push(declare_ears_part_vertical! {
                    EarMiddle {
                        pos: pos![-4.0, 8.0, anchor_z],
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
                            pos: pos![8.0, 0.0, anchor_z],
                            size: [4, 8],
                            uv: [36, 16, 4, 8],
                            back_uv: Some([12, 16, 4, 8]),
                            normal: Vec3::NEG_Z,
                            cw: true
                        }
                    });

                    result.push(declare_ears_part_vertical! {
                        EarAroundLeft {
                            pos: pos![-4.0, 0.0, anchor_z],
                            size: [4, 8],
                            uv: [36, 32, 4, 8],
                            back_uv: Some([12, 32, 4, 8]),
                            normal: Vec3::NEG_Z,
                            cw: true
                        }
                    });
                }
            }
            EarMode::Sides => {
                result.push(declare_ears_part_vertical! {
                    EarSidesLeft {
                        pos: pos![-8.0, 0.0, anchor_z],
                        size: [8, 8],
                        uv: [32, 0, 8, 8],
                        back_uv: Some([56, 36, 8, 8]),
                        normal: Vec3::NEG_Z,
                        back_cw: Some(true)
                    }
                });
                result.push(declare_ears_part_vertical! {
                    EarSidesRight {
                        pos: pos![8.0, 0.0, anchor_z],
                        size: [8, 8],
                        uv: [24, 0, 8, 8],
                        back_uv: Some([56, 28, 8, 8]),
                        normal: Vec3::NEG_Z,
                        back_cw: Some(true)
                    }
                });
            }
            EarMode::Floppy => {
                result.push(declare_ears_part_vertical! {
                    EarFloppyRight {
                        pos: pos![8.0, 0.0, 0.0],
                        size: [8, 8],
                        rot: [30.0, -90.0, 0.0],
                        rot_anchor: pos![0.0, 7.0, 0.0],
                        uv: [24, 0, 8, 8],
                        back_uv: Some([56, 28, 8, 8]),
                        normal: Vec3::X,
                        back_cw: Some(true)
                    }
                });

                result.push(declare_ears_part_vertical! {
                    EarFloppyLeft {
                        pos: pos![0.0, 0.0, 8.0],
                        size: [8, 8],
                        rot: [30.0, 90.0, 0.0],
                        rot_anchor: pos![0.0, 7.0, 0.0],
                        uv: [32, 0, 8, 8],
                        back_uv: Some([56, 36, 8, 8]),
                        normal: Vec3::NEG_X,
                        back_cw: Some(true)
                    }
                });
            }
            EarMode::Out => {
                let (pos_y, pos_z) = match anchor {
                    EarAnchor::Center => (8.0, 0.0),
                    EarAnchor::Front => (0.0, -8.0),
                    EarAnchor::Back => (0.0, 8.0),
                };

                result.push(declare_ears_part_vertical! {
                    EarOutRight {
                        pos: pos![8.0, pos_y, pos_z],
                        size: [8, 8],
                        rot: [0.0, -90.0, 0.0],
                        uv: [24, 0, 8, 8],
                        back_uv: Some([56, 28, 8, 8]),
                        normal: Vec3::X,
                        back_cw: Some(true)
                    }
                });

                result.push(declare_ears_part_vertical! {
                    EarOutLeft {
                        pos: pos![0.0, pos_y, 8.0 + pos_z],
                        size: [8, 8],
                        rot: [0.0, 90.0, 0.0],
                        uv: [32, 0, 8, 8],
                        back_uv: Some([56, 36, 8, 8]),
                        normal: Vec3::NEG_X,
                        back_cw: Some(true)
                    }
                });
            }
            EarMode::Tall => {
                let angle = 6.0;

                let mut current_angle = angle / 3.0;
                let pos = pos![
                    -8.0 + PREV_CORNER_CANARY,
                    PREV_CORNER_CANARY,
                    PREV_CORNER_CANARY
                ];

                result.push(declare_ears_part_vertical! {
                    EarTallOne {
                        pos: pos![0.0, 8.0, anchor_z],
                        rot: [current_angle, 0.0, 0.0],
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
                        pos,
                        size: [8, 4],
                        rot: [current_angle, 0.0, 0.0],
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
                        pos,
                        size: [8, 4],
                        rot: [current_angle, 0.0, 0.0],
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
                        pos,
                        size: [8, 4],
                        rot: [current_angle, 0.0, 0.0],
                        uv: [36, 0, 8, 4],
                        back_uv: Some([56, 28, 8, 4]),
                        normal: Vec3::NEG_Z,
                        cw: true,
                        back_cw: Some(false)
                    }
                });
            }
            EarMode::TallCross => {
                result.push(declare_ears_part_vertical! {
                    EarTallLeft {
                        pos: pos![1.0, 8.0, anchor_z - 3.0],
                        size: [8, 16],
                        rot: [0.0, -45.0, 0.0],
                        uv: [24, 0, 8, 16],
                        back_uv: Some([56, 28, 8, 16]),
                        normal: Vec3::NEG_X,
                        cw: true,
                        back_cw: Some(false)
                    }
                });
                result.push(declare_ears_part_vertical! {
                    EarTallRight {
                        pos: pos![1.0, 8.0, anchor_z + 3.0],
                        size: [8, 16],
                        rot: [0.0, 45.0, 0.0],
                        uv: [24, 0, 8, 16],
                        back_uv: Some([56, 28, 8, 16]),
                        normal: Vec3::X,
                        cw: true,
                        back_cw: Some(false)
                    }
                });
            }
            EarMode::Cross => {
                result.push(declare_ears_part_vertical! {
                    EarTallLeft {
                        pos: pos![1.0, 8.0, anchor_z - 3.0],
                        size: [8, 8],
                        rot: [0.0, -45.0, 0.0],
                        uv: [24, 0, 8, 8],
                        back_uv: Some([56, 28, 8, 8]),
                        normal: Vec3::NEG_X,
                        back_cw: Some(true)
                    }
                });
                result.push(declare_ears_part_vertical! {
                    EarTallRight {
                        pos: pos![1.0, 8.0, anchor_z + 3.0],
                        size: [8, 8],
                        rot: [0.0, 45.0, 0.0],
                        uv: [32, 0, 8, 8],
                        back_uv: Some([56, 36, 8, 8]),
                        normal: Vec3::X,
                        back_cw: Some(true)
                    }
                });
            }
            EarMode::Behind | EarMode::None => {}
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

            let snout_x = (8 - snout_width) as f32 / 2.0;
            let snout_y = snout_offset;
            let snout_z = -snout_depth;

            macro_rules! snout_horizontal {
                ($name: ident, $name_2: ident, $y: expr, $normal: expr, $uv_y: expr, $uv_y_2: expr) => {
                    result.push(declare_ears_part_horizontal! {
                        $name {
                            pos: pos![snout_x, snout_y + $y as f32, snout_z],
                            size: [snout_width.into(), 1],
                            uv: [0, $uv_y, snout_width.into(), 1],
                            normal: $normal,
                            double_sided: false
                        }
                    });

                    result.push(declare_ears_part_horizontal! {
                        $name_2 {
                            pos: pos![snout_x, snout_y + $y as f32, snout_z as f32 + 1.0],
                            size: [snout_width.into(), snout_depth as u16],
                            uv: [0, $uv_y_2, snout_width.into(), 1],
                            normal: $normal,
                            double_sided: false
                        }
                    });
                };
            }

            macro_rules! snout_vertical {
                ($name: ident, $name_2: ident, $depth: expr, $depth_2: expr, $x: expr, $normal: expr, $uv_y_1: expr, $uv_y_2: expr) => {
                    result.push(declare_ears_part_vertical! {
                        $name {
                            pos: pos![snout_x + $x as f32, snout_y, snout_z + 1.0],
                            rot: [0.0, 90.0, 0.0],
                            size: [$depth, snout_height.into()],
                            uv: [7, $uv_y_1, 1, snout_height.into()],
                            normal: $normal,
                            double_sided: false
                        }
                    });

                    result.push(declare_ears_part_vertical! {
                        $name_2 {
                            pos: pos![snout_x + $x as f32, snout_y, 0.0],
                            rot: [0.0, 90.0, 0.0],
                            size: [$depth_2, snout_height.into()],
                            uv: [7, $uv_y_2, 1, snout_height.into()],
                            normal: $normal,
                            double_sided: false
                        }
                    });
                };
            }

            result.push(declare_ears_part_vertical! {
                SnoutFront {
                    pos: pos![snout_x, snout_y, snout_z],
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

            snout_vertical!(
                SnoutRightFront,
                SnoutRightRest,
                1,
                snout_depth as u16 - 1,
                snout_width,
                Vec3::X,
                0,
                4
            );
            snout_vertical!(
                SnoutLeftFront,
                SnoutLeftRest,
                1,
                snout_depth as u16 - 1,
                0.0,
                Vec3::NEG_X,
                0,
                4
            );
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
fn process_pos(pos: [PositionCanary; 3], is_slim_arms: bool, last_pos: &[f32; 3]) -> [f32; 3] {
    let mut final_pos = [0.0; 3];

    for (i, pos) in pos.into_iter().enumerate() {
        final_pos[i] = pos.resolve(i, is_slim_arms, last_pos);
    }

    final_pos
}

impl<M: ArmorMaterial> PartsProvider<M> for EarsPlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        #[cfg(feature = "markers")]
        let mut markers = Vec::new();

        let empty = Vec::with_capacity(0);

        if body_part.is_layer() || body_part.is_hat_layer() {
            return empty;
        }

        if let Some(features) = context.ears_features {
            let mut features = features;
            features.claws &= !context.armor_slots.as_ref().is_some_and(|s| s.boots.is_some());
            
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
                        pos,
                        size,
                        uvs,
                        part_definition.normal,
                        #[cfg(feature = "part_tracker")]
                        Some(name.clone()),
                    );

                    #[cfg(feature = "part_tracker")]
                    {
                        part_quad.push_groups(
                            crate::parts::provider::minecraft::compute_base_part(
                                body_part,
                                is_slim_arms,
                            )
                            .get_group(),
                        );

                        part_quad.push_group(part_definition.name);
                    }

                    {
                        let rot_anchor =
                            process_pos(part_definition.rot_anchor, is_slim_arms, &last_pos.into());

                        let anchor = if part_definition.pos.contains(&PREV_CORNER_CANARY) {
                            PartAnchorInfo::default()
                        } else {
                            PartAnchorInfo::new_part_anchor_translate(body_part, is_slim_arms)
                        };

                        let anchor =
                            anchor.with_rotation_anchor(Vec3::from(pos) + Vec3::from(rot_anchor));

                        #[cfg(feature = "markers")]
                        {
                            markers.push(Marker::new(
                                format!("{name} (Translation anchor)"),
                                anchor.translation_anchor,
                            ));
                            markers.push(Marker::new(
                                format!("{name} (Rotation anchor)"),
                                anchor.rotation_anchor,
                            ));
                        }
                        part_quad.rotate(part_definition.rot.into(), Some(anchor));
                    }

                    let new_offset = part_quad
                        .get_rotation_matrix()
                        .transform_vector3(normal_offset);

                    *part_quad.position_mut() += new_offset;

                    #[cfg(feature = "markers")]
                    markers.push(Marker::new(format!("{name} (Pos [f32; 3])"), pos.into()));

                    let pos = part_quad.get_position();
                    let size = part_quad.get_size();

                    #[cfg(feature = "markers")]
                    {
                        markers.push(Marker::new(format!("{name} (Pos Vec3)"), pos));
                        markers.push(Marker::new(format!("{name} (Pos + Size)"), pos + size));
                    }

                    let old_point = if part_definition.vertical_quad {
                        [pos[0] + size[0], pos[1] + size[1] as f32, pos[2]]
                    } else {
                        [pos[0], pos[1], pos[2] - size[1] as f32]
                    };

                    let old = Vec3::from(old_point) - normal_offset;

                    last_pos = part_quad.get_rotation_matrix().transform_point3(old);

                    #[cfg(feature = "markers")]
                    {
                        markers.push(Marker::new(format!("{name} (Old point)"), old_point.into()));
                        markers.push(Marker::new(
                            format!("{name} (Rotated Old point)"),
                            part_quad.get_rotation_matrix().transform_point3(old),
                        ));
                        markers.push(Marker::new(format!("{name} (lastpos)"), last_pos));
                    }

                    #[cfg(feature = "markers")]
                    {
                        part_quad.add_markers(markers.drain(..).as_slice());
                    }

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

use ears_rs::features::EarsFeatures;

use crate::{
    model::ArmorMaterial,
    parts::part::{Part, PartAnchorInfo},
    types::{PlayerBodyPartType, PlayerPartTextureType},
};

use super::{PartsProvider, PlayerPartProviderContext};

pub struct EarsPlayerPartsProvider;

enum EarsPlayerBodyPartType {
    Chest,
    LeftArmClaw,
    RightArmClaw,
    LeftLegClaw,
    RightLegClaw,
}

#[inline(always)]
fn process_pos(pos: [f32; 3], is_slim_arms: bool) -> [f32; 3] {
    let mut pos = pos;

    for ele in pos.as_mut_slice() {
        if (*ele).abs() == ARM_PIXEL_CANARY {
            *ele = if is_slim_arms { 3.0 } else { 4.0 } * ARM_PIXEL_CANARY.signum();
        }
    }

    pos
}

macro_rules! declare_ears_parts {
    {$parts: expr, $part: expr, $is_slim_arms:expr, $(
        $ears_part: ident {
            texture: $texture: ident,
            pos: $pos: expr,
            rot: $rot: expr,
            size: $size: expr,
            uv: [$($uv: tt)*],
            enabled: $($feature: tt)*
         } $(,)*
    )+} => {
        {
            use crate::parts::uv::uv_from_pos_and_size;

            $(
                if $($feature)* {
                    let _ = EarsPlayerBodyPartType::$ears_part;
                    let pos = process_pos($pos, $is_slim_arms);
                    
                    let mut part_quad = Part::new_quad(
                        PlayerPartTextureType::$texture,
                        process_pos($pos, $is_slim_arms),
                        $size,
                        uv_from_pos_and_size($($uv)*),
                    );

                    part_quad.rotate(
                        $rot.into(),
                        Some(PartAnchorInfo::new_part_anchor_translate(
                            $part,
                            $is_slim_arms,
                        ).with_rotation_anchor(pos.into())),
                    );
                    
                    let posuwu = part_quad.get_position();
                    
                    part_quad.markers_mut().push(posuwu);

                    $parts.push(part_quad);
                }
            )+
        }
    };
}

const ARM_PIXEL_CANARY: f32 = 0xe621 as f32;
impl EarsPlayerPartsProvider {
    fn handle_body_part(
        &self,
        part: PlayerBodyPartType,
        is_slim_arms: bool,
        features: &EarsFeatures,
    ) -> Vec<Part> {
        let mut parts = Vec::new();

        if part == PlayerBodyPartType::LeftLeg {
            declare_ears_parts! { parts, part, is_slim_arms,
                LeftLegClaw {
                    texture: Skin,
                    pos: [0.0, 0.0, -4.0],
                    rot: [0.0, 0.0, 0.0],
                    size: [4, 0, 4],
                    uv: [16, 48, 4, 4],
                    enabled: features.claws
                },
            };
        } else if part == PlayerBodyPartType::RightLeg {
            declare_ears_parts! { parts, part, is_slim_arms,
                RightLegClaw {
                    texture: Skin,
                    pos: [0.0, 0.0, -4.0],
                    rot: [0.0, 0.0, 0.0],
                    size: [4, 0, 4],
                    uv: [0, 16, 4, 4],
                    enabled: features.claws
                },
            };
        } else if part == PlayerBodyPartType::LeftArm {
            declare_ears_parts! { parts, part, is_slim_arms,
                LeftArmClaw {
                    texture: Skin,
                    pos: [-ARM_PIXEL_CANARY, 0.0, 0.0],
                    rot: [90.0, 0.0, 90.0],
                    size: [4, 0, 4],
                    uv: [44, 48, 4, 4],
                    enabled: features.claws
                },
            };
        }

        parts
    }
}

impl<M: ArmorMaterial> PartsProvider<M> for EarsPlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        if body_part.is_layer() || body_part.is_hat_layer() {
            return vec![];
        }

        if let Some(features) = context.ears_features {
            let is_slim_arms = context.model.is_slim_arms();

            Self::handle_body_part(&self, body_part, is_slim_arms, &features)
        } else {
            vec![]
        }
    }
}

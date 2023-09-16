use ears_rs::features::EarsFeatures;

use crate::{
    model::ArmorMaterial,
    parts::{
        part::{Part, PartAnchorInfo},
        uv::{uv_from_pos_and_size, uv_from_pos_and_size_flipped},
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
};

use super::{PartsProvider, PlayerPartProviderContext};

pub struct EarsPlayerPartsProvider;

impl EarsPlayerPartsProvider {
    fn handle_head(
        &self,
        part: PlayerBodyPartType,
        is_slim_arms: bool,
        features: &EarsFeatures,
    ) -> Vec<Part> {
        vec![]
    }

    fn handle_body(
        &self,
        part: PlayerBodyPartType,
        is_slim_arms: bool,
        features: &EarsFeatures,
    ) -> Vec<Part> {
        vec![]
    }

    fn handle_arms(
        &self,
        part: PlayerBodyPartType,
        is_slim_arms: bool,
        features: &EarsFeatures,
    ) -> Vec<Part> {
        vec![]
    }

    fn handle_legs(
        &self,
        part: PlayerBodyPartType,
        is_slim_arms: bool,
        features: &EarsFeatures,
    ) -> Vec<Part> {
        let mut parts = Vec::new();

        if features.claws {
            let (claw_x, claw_y) = if part == PlayerBodyPartType::LeftLeg {
                (16, 48)
            } else {
                (0, 16)
            };
            
            let mut claw = Part::new_quad(
                PlayerPartTextureType::Skin,
                [0.0, 0.0, -4.0],
                [4, 0, 4],
                uv_from_pos_and_size_flipped(claw_x, claw_y, 4, 4),
            );
            claw.set_anchor(Some(PartAnchorInfo::new_part_anchor_translate(
                part,
                is_slim_arms,
            )));

            parts.push(claw);
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

            let result = match body_part {
                PlayerBodyPartType::Head => EarsPlayerPartsProvider::handle_head,
                PlayerBodyPartType::Body => EarsPlayerPartsProvider::handle_body,
                PlayerBodyPartType::LeftArm | PlayerBodyPartType::RightArm => {
                    EarsPlayerPartsProvider::handle_arms
                }
                PlayerBodyPartType::LeftLeg | PlayerBodyPartType::RightLeg => {
                    EarsPlayerPartsProvider::handle_legs
                }

                PlayerBodyPartType::HeadLayer
                | PlayerBodyPartType::BodyLayer
                | PlayerBodyPartType::LeftArmLayer
                | PlayerBodyPartType::RightArmLayer
                | PlayerBodyPartType::LeftLegLayer
                | PlayerBodyPartType::RightLegLayer => {
                    unreachable!("Gotten layer from non-layer part")
                }
            };

            (result)(self, body_part, is_slim_arms, &features)
        } else {
            vec![]
        }
    }
}

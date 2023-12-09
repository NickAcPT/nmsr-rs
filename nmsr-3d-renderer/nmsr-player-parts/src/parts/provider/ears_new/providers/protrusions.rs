use ears_rs::features::EarsFeatures;

use crate::{
    model::ArmorMaterial,
    parts::provider::{
        ears::{providers::builder::EarsModPartBuilder, EarsModPartProvider, ext::PlayerPartProviderContextExt},
        PlayerPartProviderContext,
    },
    types::PlayerBodyPartType,
};

use super::uv_utils::{TextureFlip, TextureRotation};

#[derive(Debug, Copy, Clone)]
pub(crate) struct EarsModProtrusionsPartProvider<M>(std::marker::PhantomData<M>);

impl<M: ArmorMaterial> Default for EarsModProtrusionsPartProvider<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: ArmorMaterial> EarsModPartProvider<M> for EarsModProtrusionsPartProvider<M> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool {
        matches!(
            body_part,
            PlayerBodyPartType::LeftArm
                | PlayerBodyPartType::RightArm
                | PlayerBodyPartType::LeftLeg
                | PlayerBodyPartType::RightLeg
        )
    }

    fn provides_for_feature(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
    ) -> bool {
        features.claws || features.horn
    }

    fn provide_parts(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
    ) {
        {
            if features.claws {
                if !context.is_wearing_boots() {
                    builder.stack(|b| {
                        b.anchor_to(PlayerBodyPartType::LeftLeg);
                        b.translate_i(0, 0, -4);
                        b.rotate_i(90, 1, 0, 0);
                        b.quad_double_sided(16, 48, 4, 4, TextureRotation::None, TextureFlip::Vertical, "Claw Left Leg");
                    });
                
                    builder.stack(|b| {
                        b.anchor_to(PlayerBodyPartType::RightLeg);
                        b.translate_i(0, 0, -4);
                        b.rotate_i(90, 1, 0, 0);
                        b.quad_double_sided(0, 16, 4, 4, TextureRotation::None, TextureFlip::Vertical, "Claw Right Leg");
                    });
                }
                
                builder.stack(|b| {
                    b.anchor_to(PlayerBodyPartType::LeftArm);
                    //b.rotate_i(90, 0, 1, 0);
                    //b.translate_i(-4, 0, if context.model.is_slim_arms() {3} else {4});
                    b.quad_double_sided(44, 48, 4, 4, TextureRotation::UpsideDown, TextureFlip::Horizontal, "Claw Left Arm");
                });
            
                builder.stack(|b| {
                    b.anchor_to(PlayerBodyPartType::RightArm);
                    b.rotate_i(90, 0, 1, 0);
                    b.translate_i(-4, 0, 0);
                    b.quad_double_sided(52, 16, 4, 4, TextureRotation::UpsideDown, TextureFlip::None, "Claw Right Arm");
                });
            }
        }
        
    }
}

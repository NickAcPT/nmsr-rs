use ears_rs::features::EarsFeatures;

use crate::{
    model::ArmorMaterial,
    parts::provider::{
        ears::{
            ext::PlayerPartProviderContextExt, providers::builder::EarsModPartBuilder,
            EarsModPartProvider,
        },
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
            PlayerBodyPartType::Head
                | PlayerBodyPartType::LeftArm
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
        body_part: PlayerBodyPartType,
    ) {
        claws(features, context, builder, body_part);
        horn(features, context, builder, body_part);
    }
}

fn claws<M: ArmorMaterial>(
    features: &EarsFeatures,
    context: &PlayerPartProviderContext<M>,
    builder: &mut EarsModPartBuilder<'_, M>,
    body_part: PlayerBodyPartType,
) {
    if !features.claws {
        return;
    }

    if !context.is_wearing_boots() {
        if matches!(body_part, PlayerBodyPartType::LeftLeg) {
            builder.stack(|b| {
                b.anchor_to(PlayerBodyPartType::LeftLeg);
                b.rotate_i(-90, 1, 0, 0);
                b.quad_double_sided(
                    16,
                    48,
                    4,
                    4,
                    TextureRotation::None,
                    TextureFlip::Horizontal,
                    "Claw Left Leg",
                );
            });
        }

        if matches!(body_part, PlayerBodyPartType::RightLeg) {
            builder.stack(|b| {
                b.anchor_to(PlayerBodyPartType::RightLeg);
                b.translate_i(0, 0, 0);
                b.rotate_i(-90, 1, 0, 0);
                b.quad_double_sided(
                    0,
                    16,
                    4,
                    4,
                    TextureRotation::None,
                    TextureFlip::Horizontal,
                    "Claw Right Leg",
                );
            });
        }
    }

    if matches!(body_part, PlayerBodyPartType::LeftArm) {
        builder.stack(|b| {
            b.anchor_to(PlayerBodyPartType::LeftArm);
            b.translate_i(0, -4, 0);
            b.rotate_i(-90, 0, 1, 0);
            b.quad_double_sided(
                44,
                48,
                4,
                4,
                TextureRotation::UpsideDown,
                TextureFlip::None,
                "Claw Left Arm",
            );
        });
    }

    if matches!(body_part, PlayerBodyPartType::RightArm) {
        builder.stack(|b| {
            b.anchor_to(PlayerBodyPartType::RightArm);
            b.translate_i(if context.model.is_slim_arms() { 3 } else { 4 }, -4, 4);
            b.rotate_i(90, 0, 1, 0);
            b.quad_double_sided(
                52,
                16,
                4,
                4,
                TextureRotation::UpsideDown,
                TextureFlip::None,
                "Claw Right Arm",
            );
        });
    }
}

fn horn<M: ArmorMaterial>(
    features: &EarsFeatures,
    context: &PlayerPartProviderContext<M>,
    builder: &mut EarsModPartBuilder<'_, M>,
    body_part: PlayerBodyPartType,
) {
    if !matches!(body_part, PlayerBodyPartType::Head) || !features.horn {
        return;
    }

    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);
        b.rotate_i(180, 0, 1, 0);
        b.translate_i(-8, 8, 0);
        b.rotate_i(25, 1, 0, 0);
        b.quad_double_sided(
            56,
            0,
            8,
            8,
            TextureRotation::None,
            TextureFlip::Horizontal,
            "Horn",
        );
    });
}

use ears_rs::features::{
    data::ear::{EarAnchor, EarMode},
    EarsFeatures,
};

use crate::{
    model::ArmorMaterial,
    parts::provider::{
        ears::{providers::builder::EarsModPartBuilder, EarsModPartProvider},
        PlayerPartProviderContext,
    },
    types::PlayerBodyPartType,
};

use super::uv_utils::{TextureRotation, TextureFlip};

#[derive(Debug, Copy, Clone)]
pub(crate) struct EarsModEarsPartProvider<M>(std::marker::PhantomData<M>);

impl<M: ArmorMaterial> Default for EarsModEarsPartProvider<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: ArmorMaterial> EarsModPartProvider<M> for EarsModEarsPartProvider<M> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool {
        matches!(body_part, PlayerBodyPartType::Head)
    }

    fn provides_for_feature(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
    ) -> bool {
        features.ear_mode != EarMode::None
    }

    fn provide_parts(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
    ) {
        if matches!(features.ear_mode, EarMode::Above | EarMode::Around) {
            builder.stack(|b| {
                b.anchor_to(PlayerBodyPartType::Head);

                match features.ear_anchor {
                    EarAnchor::Center => b.translate(0., 0., 4.),
                    EarAnchor::Back => b.translate(0., 0., 8.),
                    _ => {}
                }

                b.stack(|b| {
                    b.translate(-4., 8., 0.);
                    b.quad_front(24, 0, 16, 8, TextureRotation::None, TextureFlip::None, "Ears Top");
                    b.quad_back(56, 28, 16, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Top");
                });
            });
        }
    }
}

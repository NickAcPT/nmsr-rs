use ears_rs::features::{
    data::wing::WingMode,
    EarsFeatures,
};

use crate::{
    model::ArmorMaterial,
    parts::provider::{
        ears::{
            providers::builder::EarsModPartBuilder, EarsModPartProvider, PlayerPartEarsTextureType,
        },
        PlayerPartProviderContext,
    },
    types::PlayerBodyPartType,
};

use super::uv_utils::{TextureFlip, TextureRotation};

#[derive(Debug, Copy, Clone)]
pub(crate) struct EarsModWingsPartProvider<M>(std::marker::PhantomData<M>);

impl<M: ArmorMaterial> Default for EarsModWingsPartProvider<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: ArmorMaterial> EarsModPartProvider<M> for EarsModWingsPartProvider<M> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool {
        matches!(body_part, PlayerBodyPartType::Body)
    }

    fn provides_for_feature(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
    ) -> bool {
        features.wing.is_some_and(|w| w.mode != WingMode::None)
    }

    fn provide_parts(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
        body_part: PlayerBodyPartType,
    ) {
        let Some(wing) = features.wing else {
            return;
        };

        builder.stack_texture(PlayerPartEarsTextureType::Wings.into(), |b| {
            let wing_mode = wing.mode;

            b.stack(|b| {
                let wiggle = if wing.animated {
                    f32::sin(8.0 / 12.0) * 2.0
                } else {
                    0.0
                };
                b.anchor_to(PlayerBodyPartType::Body);
                b.translate_i(2, -2, 4);
                
                if wing_mode == WingMode::SymmetricDual || wing_mode == WingMode::AsymmetricL {
                    b.stack(|b| {
                        b.rotate(0.0, -120. + wiggle, 0.0);
                        b.quad_front(
                            0,
                            0,
                            20,
                            16,
                            TextureRotation::None,
                            TextureFlip::Horizontal,
                            "Wing Left",
                        );
                    });
                }
                if wing_mode == WingMode::SymmetricDual || wing_mode == WingMode::AsymmetricR {
                    b.translate_i(4, 0, 0);
                    b.stack(|b| {
                        b.rotate(0.0, -60. - wiggle, 0.0);
                        b.quad_front(
                            0,
                            0,
                            20,
                            16,
                            TextureRotation::None,
                            TextureFlip::Horizontal,
                            "Wing Right",
                        );
                    });
                }
                if wing_mode == WingMode::SymmetricSingle {
                    b.translate_i(2, 0, 0);
                    b.stack(|b| {
                        b.rotate(0.0, -90. + wiggle, 0.0);
                        b.quad_front(
                            0,
                            0,
                            20,
                            16,
                            TextureRotation::None,
                            TextureFlip::Horizontal,
                            "Wing (Single)",
                        );
                    });
                }
            });
        });
    }
}

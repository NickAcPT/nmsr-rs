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

use super::uv_utils::{TextureFlip, TextureRotation};

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
        _body_part: PlayerBodyPartType,
    ) {
        match features.ear_mode {
            EarMode::Above | EarMode::Around => ears_around_or_above(builder, features),
            EarMode::Sides => ears_sides(builder, features),
            EarMode::Floppy => ears_floppy(builder, features),
            EarMode::Cross => ears_cross(builder, features),
            EarMode::Out => ears_out(builder, features),
            EarMode::Tall => ears_tall(builder, features),
            EarMode::TallCross => ears_tall_cross(builder, features),
            EarMode::Behind => {
                unreachable!("Behind mode should have been replaced with Out mode w/ Back anchor")
            }
            EarMode::None => {}
        }
    }
}

fn ears_around_or_above<M: ArmorMaterial>(
    builder: &mut EarsModPartBuilder<'_, M>,
    features: &EarsFeatures,
) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);

        match features.ear_anchor {
            EarAnchor::Center => b.translate_i(0, 0, 4),
            EarAnchor::Back => b.translate_i(0, 0, 8),
            _ => {}
        }

        b.stack(|b| {
            b.translate_i(-4, 8, 0);
            b.quad_double_sided_complete(
                /* uv front */
                24,
                0,
                /* uv back */
                56,
                28,
                /* size */
                16,
                8,
                /* rotation front */
                TextureRotation::None,
                TextureFlip::None,
                /* rotation back */
                TextureRotation::Clockwise,
                TextureFlip::None,
                "Ears Top",
            );
        });

        if features.ear_mode == EarMode::Around {
            b.stack(|b| {
                b.translate_i(-4, 0, 0);
                b.quad_double_sided_complete(
                    /* uv front */
                    36,
                    32,
                    /* uv back */
                    12,
                    32,
                    /* size */
                    4,
                    8,
                    /* rotation front */
                    TextureRotation::Clockwise,
                    TextureFlip::None,
                    /* rotation back */
                    TextureRotation::Clockwise,
                    TextureFlip::None,
                    "Ears Left",
                );
            });

            b.stack(|b| {
                b.translate_i(8, 0, 0);
                b.quad_double_sided_complete(
                    /* uv front */
                    36,
                    16,
                    /* uv back */
                    12,
                    16,
                    /* size */
                    4,
                    8,
                    /* rotation front */
                    TextureRotation::Clockwise,
                    TextureFlip::None,
                    /* rotation back */
                    TextureRotation::Clockwise,
                    TextureFlip::None,
                    "Ears Right",
                );
            });
        }
    });
}

fn ears_sides<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);

        match features.ear_anchor {
            EarAnchor::Center => b.translate_i(0, 0, 4),
            EarAnchor::Back => b.translate_i(0, 0, 8),
            _ => {}
        }

        b.translate_i(-8, 0, 0);
        b.quad_double_sided_complete(
            /* uv front */
            32,
            0,
            /* uv back */
            56,
            36,
            /* size */
            8,
            8,
            /* rotation front */
            TextureRotation::None,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::Clockwise,
            TextureFlip::None,
            "Ears Side Left",
        );

        b.translate_i(16, 0, 0);
        b.quad_double_sided_complete(
            /* uv front */
            24,
            0,
            /* uv back */
            56,
            28,
            /* size */
            8,
            8,
            /* rotation front */
            TextureRotation::None,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::Clockwise,
            TextureFlip::None,
            "Ears Side Right",
        );
    });
}

fn ears_floppy<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);

        b.translate_i(0, -1, 8);

        b.rotate_i(90, 0, 1, 0);

        b.translate_i(0, 8, 0);
        b.rotate_i(30, 1, 0, 0);
        b.translate_i(0, -8, 0);

        b.quad_double_sided_complete(
            /* uv front */
            32,
            0,
            /* uv back */
            56,
            36,
            /* size */
            8,
            8,
            /* rotation front */
            TextureRotation::None,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::Clockwise,
            TextureFlip::None,
            "Ears Floppy Left",
        );
    });

    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);

        b.translate_i(8, -1, 0);

        b.rotate_i(-90, 0, 1, 0);

        b.translate_i(0, 8, 0);
        b.rotate_i(30, 1, 0, 0);
        b.translate_i(0, -8, 0);

        b.quad_double_sided_complete(
            /* uv front */
            24,
            0,
            /* uv back */
            56,
            28,
            /* size */
            8,
            8,
            /* rotation front */
            TextureRotation::None,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::Clockwise,
            TextureFlip::None,
            "Ears Floppy Right",
        );
    });
}

fn ears_out<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);
        b.rotate_i(90, 0, 1, 0);

        match features.ear_anchor {
            EarAnchor::Back => b.translate_i(-16, 0, 0),
            EarAnchor::Center => b.translate_i(-8, 8, 0),
            _ => {}
        }

        b.quad_double_sided_complete(
            /* uv front */
            32,
            0,
            /* uv back */
            56,
            36,
            /* size */
            8,
            8,
            /* rotation front */
            TextureRotation::None,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::Clockwise,
            TextureFlip::None,
            "Ears Out Left",
        );

        b.rotate_i(180, 0, 1, 0);
        b.translate_i(-8, 0, -8);

        b.quad_double_sided_complete(
            /* uv front */
            24,
            0,
            /* uv back */
            56,
            28,
            /* size */
            8,
            8,
            /* rotation front */
            TextureRotation::None,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::Clockwise,
            TextureFlip::None,
            "Ears Out Right",
        );
    });
}

fn ears_cross<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);

        match features.ear_anchor {
            EarAnchor::Center => b.translate_i(0, 0, 4),
            EarAnchor::Back => b.translate_i(0, 0, 8),
            _ => {}
        }

        b.translate_i(4, 8, 0);

        b.stack(|b| {
            b.rotate_i(-45, 0, 1, 0);
            b.translate_i(-4, 0, 0);
            b.quad_double_sided_complete(
                /* uv front */
                24,
                0,
                /* uv back */
                56,
                28,
                /* size */
                8,
                8,
                /* rotation front */
                TextureRotation::None,
                TextureFlip::None,
                /* rotation back */
                TextureRotation::Clockwise,
                TextureFlip::None,
                "Ears Cross Left",
            );
        });

        b.stack(|b| {
            b.rotate_i(45, 0, 1, 0);
            b.translate_i(-4, 0, 0);
            b.quad_double_sided_complete(
                /* uv front */
                32,
                0,
                /* uv back */
                56,
                36,
                /* size */
                8,
                8,
                /* rotation front */
                TextureRotation::None,
                TextureFlip::None,
                /* rotation back */
                TextureRotation::Clockwise,
                TextureFlip::None,
                "Ears Cross Right",
            );
        });
    });
}

fn ears_tall<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);
        b.translate_i(0, 8, 0);

        match features.ear_anchor {
            EarAnchor::Center => b.translate_i(0, 0, 4),
            EarAnchor::Back => b.translate_i(0, 0, 8),
            _ => {}
        }

        let ang = -6;

        b.rotate_i(ang / 3, -1, 0, 0);
        b.quad_double_sided_complete(
            /* uv front */
            24,
            0,
            /* uv back */
            56,
            40,
            /* size */
            8,
            4,
            /* rotation front */
            TextureRotation::Clockwise,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::None,
            TextureFlip::None,
            "Ears Tall 1",
        );

        b.translate_i(0, 4, 0);
        b.rotate_i(ang, -1, 0, 0);
        b.quad_double_sided_complete(
            /* uv front */
            28,
            0,
            /* uv back */
            56,
            36,
            /* size */
            8,
            4,
            /* rotation front */
            TextureRotation::Clockwise,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::None,
            TextureFlip::None,
            "Ears Tall 2",
        );

        b.translate_i(0, 4, 0);
        b.rotate_i(ang / 2, -1, 0, 0);
        b.quad_double_sided_complete(
            /* uv front */
            32,
            0,
            /* uv back */
            56,
            32,
            /* size */
            8,
            4,
            /* rotation front */
            TextureRotation::Clockwise,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::None,
            TextureFlip::None,
            "Ears Tall 3",
        );

        b.translate_i(0, 4, 0);
        b.rotate_i(ang, -1, 0, 0);
        b.quad_double_sided_complete(
            /* uv front */
            36,
            0,
            /* uv back */
            56,
            28,
            /* size */
            8,
            4,
            /* rotation front */
            TextureRotation::Clockwise,
            TextureFlip::None,
            /* rotation back */
            TextureRotation::None,
            TextureFlip::None,
            "Ears Tall 4",
        );
    });
}

fn ears_tall_cross<M: ArmorMaterial>(
    builder: &mut EarsModPartBuilder<'_, M>,
    features: &EarsFeatures,
) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);

        match features.ear_anchor {
            EarAnchor::Center => b.translate_i(0, 0, 4),
            EarAnchor::Back => b.translate_i(0, 0, 8),
            _ => {}
        }

        b.translate_i(4, 8, 0);

        b.stack(|b| {
            b.rotate_i(-45, 0, 1, 0);
            b.translate_i(-4, 0, 0);
            b.quad_double_sided_complete(
                /* uv front */
                24,
                0,
                /* uv back */
                56,
                28,
                /* size */
                8,
                16,
                /* rotation front */
                TextureRotation::Clockwise,
                TextureFlip::None,
                /* rotation back */
                TextureRotation::None,
                TextureFlip::None,
                "Ears Tall Cross Left",
            );
        });

        b.stack(|b| {
            b.rotate_i(45, 0, 1, 0);
            b.translate_i(-4, 0, 0);
            b.quad_double_sided_complete(
                /* uv front */
                24,
                0,
                /* uv back */
                56,
                28,
                /* size */
                8,
                16,
                /* rotation front */
                TextureRotation::Clockwise,
                TextureFlip::None,
                /* rotation back */
                TextureRotation::None,
                TextureFlip::None,
                "Ears Tall Cross Right",
            );
        });
    });
}

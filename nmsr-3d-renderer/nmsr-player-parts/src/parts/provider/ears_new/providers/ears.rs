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
    ) {
        match features.ear_mode {
            EarMode::Above | EarMode::Around => ears_around_or_above(builder, features),
            EarMode::Sides => ears_sides(builder, features),
            EarMode::Floppy => ears_floppy(builder, features),
            EarMode::Cross => ears_cross(builder, features),
            EarMode::Out => ears_out(builder, features),
            _ => (),
        }
    }
}

fn ears_around_or_above<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);

        match features.ear_anchor {
            EarAnchor::Center => b.translate(0., 0., 4.),
            EarAnchor::Back => b.translate(0., 0., 8.),
            _ => {}
        }
        
        #[rustfmt::skip]
        b.stack(|b| {
            b.translate(-4., 8., 0.);
            b.quad_front(24, 0, 16, 8, TextureRotation::None, TextureFlip::None, "Ears Top");
            b.quad_back(56, 28, 16, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Top");
        });

        if features.ear_mode == EarMode::Around {
            #[rustfmt::skip]
            b.stack(|b| {
                b.translate(-4., 0., 0.);
                b.quad_front(36, 32, 4, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Left");
                b.quad_back(12, 32, 4, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Left");
            });

            #[rustfmt::skip]
            b.stack(|b| {
                b.translate(8.0, 0., 0.);
                b.quad_front(36, 16, 4, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Right");
                b.quad_back(12, 16, 4, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Right");
            });
        }
    });
}

fn ears_sides<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);
        
        match features.ear_anchor {
            EarAnchor::Center => b.translate(0., 0., 4.),
            EarAnchor::Back => b.translate(0., 0., 8.),
            _ => {}
        }
        
        b.translate_i(-8, 0, 0);
        b.quad_front(32, 0, 8, 8, TextureRotation::None, TextureFlip::None, "Ears Side Left");
        b.quad_back(56, 36, 8, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Side Left");
    
        b.translate_i(16, 0, 0);
        b.quad_front(24, 0, 8, 8, TextureRotation::None, TextureFlip::None, "Ears Side Right");
        b.quad_back(56, 28, 8, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Side Right");
        
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
        
        
        b.quad_front(32, 0, 8, 8, TextureRotation::None, TextureFlip::None, "Ears Floppy Left");
        b.quad_back(56, 36, 8, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Floppy Left");
    });
    
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);
        
        b.translate_i(8, -1, 0);
        
        b.rotate_i(-90, 0, 1, 0);
        
        b.translate_i(0, 8, 0);
        b.rotate_i(30, 1, 0, 0);
        b.translate_i(0, -8, 0);
        
        b.quad_front(24, 0, 8, 8,TextureRotation::None, TextureFlip::None, "Ears Floppy Right");
        b.quad_back(56, 28, 8, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Floppy Right");
    });
}

fn ears_out<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);
        b.rotate_i(90, 0, 1, 0);
        if features.ear_anchor == EarAnchor::Back {
            b.translate_i(-16, 0, 0);
        } else if features.ear_anchor == EarAnchor::Center {
            b.translate_i(-8, 8, 0);
        } else if features.ear_anchor == EarAnchor::Front {
            b.translate_i(0, 0, 0);
        }
        
        b.quad_front(32, 0, 8, 8, TextureRotation::None, TextureFlip::None, "Ears Out Left");
        b.quad_back(56, 36, 8, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Out Left");
        
        b.rotate_i(180, 0, 1, 0);
        b.translate_i(-8, 0, -8);
        
        b.quad_front(24, 0, 8, 8, TextureRotation::None, TextureFlip::None, "Ears Out Right");
        b.quad_back(56, 28, 8, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Out Right");
    });
}

fn ears_cross<M: ArmorMaterial>(builder: &mut EarsModPartBuilder<'_, M>, features: &EarsFeatures) {
    builder.stack(|b| {
        b.anchor_to(PlayerBodyPartType::Head);
        if features.ear_anchor == EarAnchor::Center {
            b.translate_i(0, 0, 4);
        } else if features.ear_anchor == EarAnchor::Back {
            b.translate_i(0, 0, 8);
        }
        b.translate_i(4, 8, 0);
        
        b.stack(|b| {
            b.rotate_i(-45, 0, 1, 0);
            b.translate_i(-4, 0, 0);
            b.quad_front(24, 0, 8, 8, TextureRotation::None, TextureFlip::None, "Ears Cross Left");
            b.quad_back(56, 28, 8, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Cross Left");
        });
        
        b.stack(|b| {
            b.rotate_i(45, 0, 1, 0);
            b.translate_i(-4, 0, 0);
            b.quad_front(32, 0, 8, 8, TextureRotation::None, TextureFlip::None, "Ears Cross Right");
            b.quad_back(56, 36, 8, 8, TextureRotation::Clockwise, TextureFlip::None, "Ears Cross Right");
        });
    });
}
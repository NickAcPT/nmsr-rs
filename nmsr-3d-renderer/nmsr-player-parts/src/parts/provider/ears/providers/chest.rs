use ears_rs::features::EarsFeatures;

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
pub(crate) struct EarsModChestPartProvider<M>(std::marker::PhantomData<M>);

impl<M: ArmorMaterial> Default for EarsModChestPartProvider<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: ArmorMaterial> EarsModPartProvider<M> for EarsModChestPartProvider<M> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool {
        matches!(
            body_part,
            PlayerBodyPartType::Body | PlayerBodyPartType::BodyLayer
        )
    }

    fn provides_for_feature(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
    ) -> bool {
        features.chest_size > 0.0
    }

    fn provide_parts(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
        body_part: PlayerBodyPartType,
    ) {
        let chest_suffix = if body_part == PlayerBodyPartType::BodyLayer {
            " (Layer)"
        } else {
            ""
        };

        let chest_size = features.chest_size;
        let is_chest_layer = body_part == PlayerBodyPartType::BodyLayer;

        builder.stack(|b| {
            b.anchor_to(PlayerBodyPartType::Body);
            b.translate_i(0, 10, 0);

            b.rotate_f(180.0, 0, 0, 1);
            b.translate_i(-8, 0, 0);

            b.rotate_f(-chest_size * 45.0, 1, 0, 0);

            b.stack(|b| {
                do_chest_grow(body_part, b);

                if !is_chest_layer {
                    b.quad_front(
                        20,
                        22,
                        8,
                        4,
                        TextureRotation::None,
                        TextureFlip::Both,
                        "Chest (Top)".to_owned() + chest_suffix,
                    );
                } else {
                    b.stack(|b| {
                        b.quad_front(
                            0,
                            48,
                            4,
                            4,
                            TextureRotation::None,
                            TextureFlip::Both,
                            "Chest (Top Left)".to_owned() + chest_suffix,
                        );
                        b.translate_i(4, 0, 0);
                        b.quad_front(
                            12,
                            48,
                            4,
                            4,
                            TextureRotation::None,
                            TextureFlip::Both,
                            "Chest (Top Right)".to_owned() + chest_suffix,
                        );
                    });
                }
            });

            b.stack(|b| {
                b.translate_i(0, 4, 0);
                b.rotate_i(90, 1, 0, 0);

                do_chest_grow(body_part, b);

                let (u, v) = if is_chest_layer { (28, 48) } else { (56, 44) };

                b.quad_front(
                    u,
                    v,
                    8,
                    4,
                    TextureRotation::None,
                    TextureFlip::Both,
                    "Chest (Bottom)".to_owned() + chest_suffix,
                );
            });

            b.stack(|b| {
                b.rotate_i(90, 0, 1, 0);
                b.translate(-4.0, 0.0, 0.01f32);

                let (u, v) = if is_chest_layer { (48, 48) } else { (60, 48) };
                
                b.stack(|b| {
                    do_chest_grow(body_part, b);
                    
                    if body_part == PlayerBodyPartType::BodyLayer {
                        b.translate(0.25, 0., 0.);
                    }
                    
                    b.quad_front(
                        u,
                        v,
                        4,
                        4,
                        TextureRotation::None,
                        TextureFlip::Both,
                        "Chest (Right)".to_owned() + chest_suffix,
                    );
                });

                b.translate(0.0, 0.0, 7.98f32);
                b.rotate_i(180, 0, 1, 0);
                b.translate_i(-4, 0, 0);
                
                do_chest_grow(body_part, b);

                b.quad_front(
                    u,
                    v,
                    4,
                    4,
                    TextureRotation::None,
                    TextureFlip::Vertical,
                    "Chest (Left)".to_owned() + chest_suffix,
                );
            });
        });
    }
}

fn do_chest_grow<M: ArmorMaterial>(
    body_part: PlayerBodyPartType,
    b: &mut EarsModPartBuilder<'_, M>,
) {
    if body_part != PlayerBodyPartType::BodyLayer {
        return;
    }

    b.translate_i(4, 2, 0);
    b.scale(8.5 / 8.0, 4.5 / 4.0, 1.0);
    b.translate_i(-4, -2, 0);

    b.translate(0., 0., -0.25);
}

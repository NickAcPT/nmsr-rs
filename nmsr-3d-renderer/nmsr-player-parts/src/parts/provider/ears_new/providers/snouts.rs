use ears_rs::features::EarsFeatures;

use crate::{
    model::ArmorMaterial,
    parts::provider::{
        ears::{
            providers::builder::EarsModPartBuilder,
            EarsModPartProvider,
        },
        PlayerPartProviderContext,
    },
    types::PlayerBodyPartType,
};

use super::uv_utils::{TextureFlip, TextureRotation};

#[derive(Debug, Copy, Clone)]
pub(crate) struct EarsModSnoutsPartProvider<M>(std::marker::PhantomData<M>);

impl<M: ArmorMaterial> Default for EarsModSnoutsPartProvider<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: ArmorMaterial> EarsModPartProvider<M> for EarsModSnoutsPartProvider<M> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool {
        matches!(body_part, PlayerBodyPartType::Head)
    }

    fn provides_for_feature(
        &self,
        feature: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
    ) -> bool {
        feature.snout.is_some()
    }

    fn provide_parts(
        &self,
        feature: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
        body_part: PlayerBodyPartType,
    ) {
        let Some(snout) = feature.snout else {
            return;
        };

        let snout_offset = snout.offset as i32;
        let snout_width = snout.width as i32;
        let snout_height = snout.height as i32;
        let snout_depth = snout.depth as i32;

        let snout_depth_minus_one = snout_depth - 1;

        builder.stack_mesh("Snout", |b| {
            b.anchor_to(PlayerBodyPartType::Head);
            b.translate(
                (8.0 - snout_width as f32) / 2f32,
                snout_offset as f32,
                -snout_depth as f32,
            );
            b.quad_front(
                0,
                2,
                snout_width as u16,
                snout_height as u16,
                TextureRotation::None,
                TextureFlip::None,
                "Snout Front",
            );

            // Top
            b.stack(|b| {
                b.translate_i(0, snout_height, 0);
                b.rotate_i(90, 1, 0, 0);
                b.quad_front(
                    0,
                    1,
                    snout_width as u16,
                    1,
                    TextureRotation::None,
                    TextureFlip::None,
                    "Snout Top (A)",
                );

                if snout_depth_minus_one > 0 {
                    b.translate_i(0, 1, 0);
                    b.scale_i(1, snout_depth_minus_one, 1);
                    b.quad_front(
                        0,
                        0,
                        snout_width as u16,
                        1,
                        TextureRotation::None,
                        TextureFlip::None,
                        "Snout Top (B)",
                    );
                }
            });

            // Bottom
            b.stack(|b| {
                b.rotate_i(90, 1, 0, 0);
                b.quad_back(
                    0,
                    (2 + snout_height) as u16,
                    snout_width as u16,
                    1,
                    TextureRotation::None,
                    TextureFlip::None,
                    "Snout Bottom (A)",
                );

                if snout_depth_minus_one > 0 {
                    b.translate_i(0, 1, 0);
                    b.scale_i(1, snout_depth_minus_one, 1);
                    b.quad_back(
                        0,
                        (2 + snout_height + 1) as u16,
                        snout_width as u16,
                        1,
                        TextureRotation::None,
                        TextureFlip::None,
                        "Snout Bottom (B)",
                    );
                }
            });
            
            b.stack(|b| {
                b.rotate_i(90, 0, 1, 0);
                
                // left
                b.stack(|b| {
                    b.translate_i(-1, 0, 0);
                    b.quad_front(7, 0, 1, snout_height as u16, TextureRotation::None, TextureFlip::None, "Snout Left (A)");
                    
                    if snout_depth_minus_one > 0 {
                        b.translate_i(-snout_depth_minus_one, 0, 0);
                        b.scale_i(snout_depth_minus_one, 1, 1);
                        b.quad_front(7, 4, 1, snout_height as u16, TextureRotation::None, TextureFlip::None, "Snout Left (B)");
                    }
                });
                
                // right
                b.stack(|b| {
                    b.translate_i(-1, 0, snout_width);
                    b.quad_back(7, 0, 1, snout_height as u16, TextureRotation::None, TextureFlip::None, "Snout Right (A)");
                    
                    if snout_depth_minus_one > 0 {
                        b.translate_i(-snout_depth_minus_one, 0, 0);
                        b.scale_i(snout_depth_minus_one, 1, 1);
                        b.quad_back(7, 4, 1, snout_height as u16, TextureRotation::None, TextureFlip::None, "Snout Right (B)");
                    }
                });
            });
        });
    }
}

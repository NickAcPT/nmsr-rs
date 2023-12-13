use ears_rs::features::{data::tail::TailMode, EarsFeatures};

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
pub(crate) struct EarsModTailsPartProvider<M>(std::marker::PhantomData<M>);

impl<M: ArmorMaterial> Default for EarsModTailsPartProvider<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: ArmorMaterial> EarsModPartProvider<M> for EarsModTailsPartProvider<M> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool {
        matches!(body_part, PlayerBodyPartType::Body)
    }

    fn provides_for_feature(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
    ) -> bool {
        features.tail.is_some_and(|t| t.mode != TailMode::None)
    }

    fn provide_parts(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
        body_part: PlayerBodyPartType,
    ) {
        let Some(tail) = features.tail.as_ref() else {
            return;
        };

        let tail_mode = tail.mode;
        let [bend_0, bend_1, bend_2, bend_3] = tail.bends;

        let (ang, swing) = if tail_mode == TailMode::Down {
            (30, 40)
        } else if tail_mode == TailMode::Back {
            (if bend_0 != 0.0 { 90 } else { 80 }, 20)
        } else if tail_mode == TailMode::Up {
            (130, -20)
        } else {
            (0, 0)
        };

        let base_angle = tail.bends[0];
        
        builder.stack(|b| {
            b.anchor_to(PlayerBodyPartType::Body);
            b.translate_i(0, 2, 4);
            
            b.rotate_i(180, 0, 0, 1);
            b.translate_i(-8, 0, 0);
            
            b.rotate_i(ang, 1, 0, 0);
            let vert = tail_mode == TailMode::Vertical;
            
            if vert {
                b.translate_i(4, 0, 0);
                b.rotate_i(90, 0, 0, 1);
                if base_angle < 0.0 {
                    b.translate_i(4, 0, 0);
                    b.rotate(0., base_angle, 0.);
                    b.translate_i(-4, 0, 0);
                }
                b.translate_i(-4, 0, 0);
                if base_angle > 0.0 {
                    b.rotate(0., base_angle, 0.);
                }
                b.rotate_i(90, 1, 0, 0);
            }

            let segments = tail.segments.max(1) as usize;

            let angles = [
                (if vert { 0.0 } else { base_angle }),
                bend_1,
                bend_2,
                bend_3,
            ];

            let seg_height = 12 / segments;
            
            for i in 0..segments {
                b.rotate(angles[i], 0., 0.);
                b.quad_front(
                    56,
                    (16 + (i * seg_height)) as u16,
                    8,
                    seg_height as u16,
                    TextureRotation::None,
                    TextureFlip::Vertical,
                    format!("Tail Segment {}", i)
                );
                b.translate_i(0, seg_height as i32, 0);
            }
        });
    }
}

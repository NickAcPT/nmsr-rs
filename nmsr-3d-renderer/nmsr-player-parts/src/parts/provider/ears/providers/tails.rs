use ears_rs::features::{data::tail::TailMode, EarsFeatures};

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

        let (mut ang, swing) = if tail_mode == TailMode::Down {
            (30, 40)
        } else if matches!(tail_mode, TailMode::Back | TailMode::Cross | TailMode::CrossOverlap | TailMode::Star | TailMode::StarOverlap) {
            (if bend_0 != 0.0 { 90 } else { 80 }, 20)
        } else if tail_mode == TailMode::Up {
            (130, -20)
        } else {
            (0, 0)
        };

        let mut base_angle = tail.bends[0];
        
        if context.movement.is_gliding {
            base_angle = -30.0;
            ang = 0;
        }
        
        let swing_amount = context.movement.limb_swing;

        builder.stack(|b| {
            b.anchor_to(PlayerBodyPartType::Body);
            b.translate_i(0, 2, 4);

            b.rotate_i(180, 0, 0, 1);
            b.translate_i(-8, 0, 0);
            
            let swing_rot = swing_amount * (swing as f32);
            let time_offset = f32::sin(context.movement.time / 12.) * 4.;

            b.rotate_i(ang, 1, 0, 0);
            b.rotate(swing_rot + time_offset, 0., 0.);
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
                let ofs = if i != 0 && matches!(tail_mode, TailMode::CrossOverlap | TailMode::StarOverlap) {
                    4
                } else {
                    0
                };
                
                b.rotate(angles[i] * (1.-(swing_amount/2.0)), 0., 0.);
                b.quad_back(
                    56,
                    (16 + (i * seg_height)) as u16,
                    8,
                    seg_height as u16,
                    TextureRotation::None,
                    TextureFlip::Both,
                    format!("Tail Segment {}", i),
                );
                
                if let TailMode::Cross | TailMode::CrossOverlap = tail_mode {
                    b.stack(|b| {
                        b.translate(4.0, 0., 0.);
                        b.rotate(0., 90.0, 0.);
                        b.translate(-4.0, -(ofs as f32), 0.0);
                        
                        b.quad_back(
                            56,
                            (16 + (i * seg_height).saturating_sub(ofs)) as u16,
                            8,
                            (seg_height + ofs) as u16,
                            TextureRotation::None,
                            TextureFlip::Both,
                            format!("Tail Cross Segment {}", i),
                        );
                    });
                } else if let TailMode::Star | TailMode::StarOverlap = tail_mode {
                    for j in 0..4 {
                        b.stack(|b| {
                            b.translate(4.0, 0., 0.);
                            b.rotate(0., 45.0 * (j + 1) as f32, 0.);
                            b.translate(-4.0, -(ofs as f32), 0.0);
                            
                            b.quad_back(
                                56,
                                (16 + (i * seg_height).saturating_sub(ofs)) as u16,
                                8,
                                (seg_height + ofs) as u16,
                                TextureRotation::None,
                                TextureFlip::Both,
                                format!("Tail Segment {} Star {}", i, j),
                            );
                        });
                    }
                }
                
                b.translate_i(0, seg_height as i32, 0);
            }
        });
    }
}

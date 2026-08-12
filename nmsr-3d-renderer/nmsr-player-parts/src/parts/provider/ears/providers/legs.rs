use ears_rs::features::{data::leg::LegMode, EarsFeatures};

use crate::{
    model::ArmorMaterial,
    parts::provider::{
        ears::{
            ext::PlayerPartProviderContextExt, providers::builder::EarsModPartBuilder,
            EarsModPartProvider, PlayerPartEarsTextureType,
        },
        PlayerPartProviderContext,
    },
    types::PlayerBodyPartType,
};

use super::uv_utils::{TextureFlip, TextureRotation};

#[derive(Debug, Copy, Clone)]
pub(crate) struct EarsModLegsPartProvider<M>(std::marker::PhantomData<M>);

impl<M: ArmorMaterial> Default for EarsModLegsPartProvider<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: ArmorMaterial> EarsModPartProvider<M> for EarsModLegsPartProvider<M> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool {
        matches!(
            body_part,
            PlayerBodyPartType::LeftLeg
                | PlayerBodyPartType::LeftLegLayer
                | PlayerBodyPartType::RightLeg
                | PlayerBodyPartType::RightLegLayer
        )
    }

    fn provides_for_feature(
        &self,
        features: &EarsFeatures,
        _context: &PlayerPartProviderContext<M>,
    ) -> bool {
        features.leg_mode != LegMode::Plantigrade
    }

    fn provide_parts(
        &self,
        features: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
        body_part: PlayerBodyPartType,
    ) {
        let full = features.leg_mode == LegMode::DigitigradeFull;
        let (leg, u, v, grow, side) = match body_part {
            PlayerBodyPartType::LeftLeg => (PlayerBodyPartType::LeftLeg, 16, 48, 0.0, "Left"),
            PlayerBodyPartType::LeftLegLayer => (PlayerBodyPartType::LeftLeg, 0, 48, 0.25, "Left"),
            PlayerBodyPartType::RightLeg => (PlayerBodyPartType::RightLeg, 0, 16, 0.0, "Right"),
            PlayerBodyPartType::RightLegLayer => {
                (PlayerBodyPartType::RightLeg, 0, 32, 0.25, "Right")
            }
            _ => return,
        };
        let enabled = !context.is_wearing_leggings();

        builder.stack_mesh(format!("Digitigrade Leg {body_part:?}"), |b| {
            b.stack_texture(PlayerPartEarsTextureType::DisplacedSkin.into(), |b| {
                b.anchor_to(leg);
                if full && enabled {
                    b.translate(0.0, 0.0, -0.5);
                }
                if full {
                    draw_digitigrade_leg(b, u, v, grow, enabled, false, true, side);
                }
                draw_digitigrade_leg(b, u, v, grow, enabled, true, !full, side);
            });
        });
    }
}

fn draw_digitigrade_leg<M: ArmorMaterial>(
    builder: &mut EarsModPartBuilder<'_, M>,
    u: u16,
    v: u16,
    grow: f32,
    enabled: bool,
    bottom: bool,
    mend: bool,
    side: &str,
) {
    let grow_twice = grow * 2.0;
    builder.stack(|b| {
        b.translate(2.0, -6.0, 2.0);
        if grow > 0.0 {
            b.translate(0.0, -grow_twice, 0.0);
        }
        b.scale(
            (4.0 + grow_twice) / 4.0,
            (12.0 + grow_twice) / 12.0,
            (4.0 + grow_twice) / 4.0,
        );
        b.translate(-2.0, 6.0, -2.0);

        let skew = f32::from(enabled);
        let vo = if bottom { 10 } else { 4 };
        let section = if bottom { "Bottom" } else { "Top" };

        b.stack(|b| {
            b.translate_i(0, if bottom { 0 } else { 6 }, 0);
            draw_side(
                b,
                u + 4,
                v + vo,
                0.0,
                0.0,
                skew,
                enabled,
                bottom,
                mend,
                side,
                section,
                "Front",
            );
            b.rotate_i(-90, 0, 1, 0);
            b.translate_i(0, 0, -4);
            draw_side(
                b,
                u,
                v + vo,
                skew,
                0.0,
                0.0,
                enabled,
                bottom,
                mend,
                side,
                section,
                "Right",
            );
            b.rotate_i(-90, 0, 1, 0);
            b.translate_i(0, 0, -4);
            draw_side(
                b,
                u + 12,
                v + vo,
                0.0,
                0.0,
                -skew,
                enabled,
                bottom,
                mend,
                side,
                section,
                "Back",
            );
            b.rotate_i(-90, 0, 1, 0);
            b.translate_i(0, 0, -4);
            draw_side(
                b,
                u + 8,
                v + vo,
                -skew,
                0.0,
                0.0,
                enabled,
                bottom,
                mend,
                side,
                section,
                "Left",
            );
        });

        b.stack(|b| {
            b.rotate_i(-90, 1, 0, 0);
            b.translate_i(0, -4, 0);
            if bottom {
                b.translate(0.0, skew, 0.0);
                b.quad_front(
                    u + 8,
                    v,
                    4,
                    4,
                    TextureRotation::None,
                    TextureFlip::Vertical,
                    format!("Digitigrade {side} Leg {section} Bottom"),
                );
            } else {
                b.translate(0.0, -skew, 12.0);
                b.quad_back(
                    u + 8,
                    v,
                    4,
                    4,
                    TextureRotation::None,
                    TextureFlip::Vertical,
                    format!("Digitigrade {side} Leg {section} Top"),
                );
            }
            if enabled && mend && bottom {
                b.translate_i(0, 1, 6);
                b.quad_front(
                    u + 4,
                    v + vo,
                    4,
                    1,
                    TextureRotation::None,
                    TextureFlip::None,
                    format!("Digitigrade {side} Leg {section} Front Fill"),
                );
                b.translate_i(0, -3, 0);
                b.quad_back(
                    u + 12,
                    v + vo,
                    4,
                    1,
                    TextureRotation::None,
                    TextureFlip::None,
                    format!("Digitigrade {side} Leg {section} Back Fill"),
                );
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_side<M: ArmorMaterial>(
    builder: &mut EarsModPartBuilder<'_, M>,
    u: u16,
    v: u16,
    x_skew: f32,
    y_skew: f32,
    z_skew: f32,
    enabled: bool,
    bottom: bool,
    mend: bool,
    side: &str,
    section: &str,
    face: &str,
) {
    let suffix = format!("Digitigrade {side} Leg {section} {face}");
    if enabled && !mend && bottom {
        builder.stack(|b| {
            b.translate_i(0, 0, 0);
            b.quad_front_skew(
                u,
                v + 2,
                4,
                4,
                x_skew,
                y_skew,
                z_skew,
                TextureRotation::None,
                TextureFlip::None,
                &suffix,
            );
        });
    } else {
        builder.quad_front_skew(
            u,
            v,
            4,
            6,
            x_skew,
            y_skew,
            z_skew,
            TextureRotation::None,
            TextureFlip::None,
            suffix.clone(),
        );
    }
    if enabled && mend && !bottom {
        builder.stack(|b| {
            b.translate_i(0, -2, 0);
            b.quad_front_skew(
                u,
                v + 6,
                4,
                2,
                -x_skew,
                -y_skew,
                -z_skew,
                TextureRotation::None,
                TextureFlip::None,
                suffix,
            );
        });
    }
}

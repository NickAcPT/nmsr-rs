use std::marker::PhantomData;

use glam::Vec3;

use crate::model::{ArmorMaterial, PlayerArmorSlot, PlayerArmorSlots};
use crate::parts::part::{Part, PartAnchorInfo};
use crate::parts::provider::{PartsProvider, PlayerPartProviderContext};
use crate::parts::uv::uv_from_pos_and_size;
use crate::types::PlayerBodyPartType::*;
use crate::types::{PlayerBodyPartType, PlayerPartTextureType};

pub struct MinecraftPlayerPartsProvider<M>(PhantomData<[M; 4]>);

impl<M> Default for MinecraftPlayerPartsProvider<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

macro_rules! body_part {
    // Matcher on many body parts
    {pos: $pos: tt, size: $size: tt, box_uv_start: ($uv_x: expr, $uv_y: expr), name: $name: expr} => {
        body_part! {
            pos: $pos,
            size: $size,
            box_uv_start: ($uv_x, $uv_y),
            texture_type: Skin,
            name: $name
        }
    };
    {pos: $pos: tt, size: $size: tt, box_uv_start: ($uv_x: expr, $uv_y: expr), texture_type: $texture_type: ident, name: $name: expr} => {
        {
            let part = Part::new_cube(
                crate::types::PlayerPartTextureType::$texture_type,
                $pos,
                $size,
                crate::parts::uv::box_uv($uv_x, $uv_y, $size),
                #[cfg(feature = "part_tracker")] Some($name.to_string()),
            );


            #[cfg(feature = "part_tracker")]
            {
                part.with_group($name)
            }
            #[cfg(not(feature = "part_tracker"))]
            {
                part
            }
        }
    };
}

impl<M: ArmorMaterial> PartsProvider<M> for MinecraftPlayerPartsProvider<M> {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        if body_part.is_layer() && !context.has_layers
            || body_part.is_hat_layer() && !context.has_hat_layer
        {
            return vec![];
        }

        let non_layer_body_part_type = body_part.get_non_layer_part();

        let part = compute_base_part(non_layer_body_part_type, context.model.is_slim_arms());

        if body_part.is_layer() || body_part.is_hat_layer() {
            let expand_offset = get_layer_expand_offset(non_layer_body_part_type);
            let box_uv_offset: (i32, i32) = get_body_part_layer_uv_offset(non_layer_body_part_type);

            return vec![expand_player_body_part(
                non_layer_body_part_type,
                part,
                expand_offset,
                box_uv_offset,
            )];
        }

        let mut result = vec![part];

        if body_part == Body && context.has_cape {
            append_cape_part(&mut result);
        }

        if body_part == Head {
            if let Some(shadow_y_pos) = context.shadow_y_pos {
                let shadow = Part::new_quad(
                    PlayerPartTextureType::Shadow,
                    [-8.0, shadow_y_pos, -8.0],
                    [16, 0, 16],
                    uv_from_pos_and_size(0, 0, 128, 128),
                    Vec3::Y,
                    #[cfg(feature = "part_tracker")]
                    Some("Shadow".to_string()),
                );
                // TODO: Expand shadow if there's armor on the feet

                result.push(shadow);
            }
        }

        if let Some(armor_slots) = &context.armor_slots {
            let part_slots =
                PlayerArmorSlots::<()>::get_armor_slots_for_part(&non_layer_body_part_type);

            for slot in part_slots {
                if let Some(armor_slot) = armor_slots.get_armor_slot(slot) {
                    if let Some(texture) = M::get_texture_type(slot) {
                        let amount = slot.get_offset();
                        let mut armor_part =
                            compute_base_part(non_layer_body_part_type, false).expand_splat(amount);

                        if slot == PlayerArmorSlot::Chestplate
                            && non_layer_body_part_type != PlayerBodyPartType::Body
                        {
                            armor_part = armor_part.expand([0.0, 0.0, 0.05].into());
                        }
                        
                        #[cfg(feature = "part_tracker")]
                        {
                            let name = armor_part.get_name_mut();
                            
                            if let Some(old_name) = name.take() {
                                name.replace(format!("{} {}", old_name, slot));
                            }
                        }

                        armor_part.set_texture(texture);
                        result.push(armor_part);
                    }
                }
            }
        }

        result
    }
}

#[cfg(feature = "part_tracker")]
pub fn get_part_group_name(non_layer_body_part_type: PlayerBodyPartType) -> &'static str {
    match non_layer_body_part_type {
        Head => "Head",
        Body => "Body",
        LeftArm => "Left Arm",
        RightArm => "Right Arm",
        LeftLeg => "Left Leg",
        RightLeg => "Right Leg",
        _ => unreachable!(
            "Tried to compute group name for unknown part {:?}",
            non_layer_body_part_type
        ),
    }
}

pub fn compute_base_part(non_layer_body_part_type: PlayerBodyPartType, is_slim_arms: bool) -> Part {
    match non_layer_body_part_type {
        Head => body_part! {
            pos: [-4, 24, -4],
            size: [8, 8, 8],
            box_uv_start: (8, 8),
            name: get_part_group_name(Head)
        },
        Body => body_part! {
            pos: [-4, 12, -2],
            size: [8, 12, 4],
            box_uv_start: (20, 20),
            name: get_part_group_name(Body)
        },
        LeftArm => {
            if is_slim_arms {
                body_part! {
                    pos: [-7, 12, -2],
                    size: [3, 12, 4],
                    box_uv_start: (36, 52),
                    name: get_part_group_name(LeftArm)
                }
            } else {
                body_part! {
                    pos: [-8, 12, -2],
                    size: [4, 12, 4],
                    box_uv_start: (36, 52),
                    name: get_part_group_name(LeftArm)
                }
            }
        }
        RightArm => {
            if is_slim_arms {
                body_part! {
                    pos: [4, 12, -2],
                    size: [3, 12, 4],
                    box_uv_start: (44, 20),
                    name: get_part_group_name(RightArm)
                }
            } else {
                body_part! {
                    pos: [4, 12, -2],
                    size: [4, 12, 4],
                    box_uv_start: (44, 20),
                    name: get_part_group_name(RightArm)
                }
            }
        }
        LeftLeg => body_part! {
            pos: [-4, 0, -2],
            size: [4, 12, 4],
            box_uv_start: (20, 52),
            name: get_part_group_name(LeftLeg)
        },
        RightLeg => body_part! {
            pos: [0, 0, -2],
            size: [4, 12, 4],
            box_uv_start: (4, 20),
            name: get_part_group_name(RightLeg)
        },
        _ => unreachable!("Got layer body part type when getting non-layer body part type."),
    }
}

pub(crate) fn perform_arm_part_rotation(
    non_layer_body_part_type: PlayerBodyPartType,
    part: &mut Part,
    is_slim_arms: bool,
    arm_rotation_angle: f32,
) {
    let normal_part = compute_base_part(non_layer_body_part_type, is_slim_arms);
    let normal_part_size = normal_part.get_size();

    if non_layer_body_part_type == LeftArm {
        let anchor = normal_part.get_position() + normal_part_size * Vec3::new(1.0, 1.0, 0.5);

        part.rotate(
            [0.0, 0.0, -arm_rotation_angle].into(),
            Some(PartAnchorInfo::new_rotation_anchor_position(anchor)),
        );
    } else if non_layer_body_part_type == RightArm {
        let anchor = normal_part.get_position() + normal_part_size * Vec3::new(0.0, 1.0, 0.5);

        part.rotate(
            [0.0, 0.0, arm_rotation_angle].into(),
            Some(PartAnchorInfo::new_rotation_anchor_position(anchor)),
        );
    }
}

fn append_cape_part(result: &mut Vec<Part>) {
    let mut cape = body_part! {
        pos: [-5, 8, 1],
        size: [10, 16, 1],
        box_uv_start: (1, 1),
        texture_type: Cape,
        name: "Cape"
    };

    cape.rotate(
        [5.0, 180.0, 0.0].into(),
        Some(PartAnchorInfo::new_rotation_anchor_position(
            [0.0, 24.0, 2.0].into(),
        )),
    );

    result.push(cape);
}

fn expand_player_body_part(
    non_layer_body_part_type: PlayerBodyPartType,
    part: Part,
    expand_offset: f32,
    box_uv_offset: (i32, i32),
) -> Part {
    let mut new_part = part.expand_splat(expand_offset);
    if let Part::Quad { .. } = new_part {
        unreachable!("Got quad when expanding body part.")
    } else if let Part::Cube {
        ref mut face_uvs,
        #[cfg(feature = "part_tracker")]
        ref mut part_tracking_data,
        ..
    } = new_part
    {
        #[cfg(feature = "part_tracker")]
        {
            let name_mut = part_tracking_data.name_mut();

            if let Some(old_name) = name_mut.take() {
                name_mut.replace(format!("{} Layer", old_name));
            }
        }

        let current_box_uv = face_uvs.north.top_left;

        let size = part.get_size();
        *face_uvs = crate::parts::uv::box_uv(
            (current_box_uv.x as i32 + box_uv_offset.0) as u16,
            (current_box_uv.y as i32 + box_uv_offset.1) as u16,
            [size.x as u16, size.y as u16, size.z as u16],
        )
    }
    new_part
}

fn get_body_part_layer_uv_offset(non_layer_body_part_type: PlayerBodyPartType) -> (i32, i32) {
    match non_layer_body_part_type {
        Head => (32, 0),
        Body => (0, 16),
        LeftArm => (16, 0),
        RightArm => (0, 16),
        LeftLeg => (-16, 0),
        RightLeg => (0, 16),
        _ => unreachable!(
            "Tried to compute UV offset for unknown part {:?}",
            non_layer_body_part_type
        ),
    }
}

fn get_layer_expand_offset(body_part: PlayerBodyPartType) -> f32 {
    match body_part {
        Head => 0.5,
        _ => 0.25,
    }
}

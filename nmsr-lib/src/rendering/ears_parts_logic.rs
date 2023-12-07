use crate::parts::player_model::PlayerModel;
use ears_rs::features::data::ear::{EarAnchor, EarMode};
use ears_rs::features::EarsFeatures;

pub(crate) fn get_ears_parts(features: &EarsFeatures, model: &PlayerModel) -> Vec<String> {
    let mut parts = vec![];

    let anchor = &features.ear_anchor;
    let mode = &features.ear_mode;
    parts.push(get_ear_anchor_key(mode, anchor));

    // Around implicitly requires the above part
    if mode == &EarMode::Around {
        parts.push(get_ear_anchor_key(&EarMode::Above, anchor));
    }

    if features.horn {
        parts.push("Horn".to_string());
    }

    if features.claws {
        parts.extend(
            ["LeftLegClaw", "RightLegClaw"]
                .map(|s| s.to_string())
                .to_vec(),
        );

        parts.push(format!("{}LeftArmClaw", model.get_dir_name()));
        parts.push(format!("{}RightArmClaw", model.get_dir_name()));
    }

    parts
}

fn get_ear_anchor_key(mode: &EarMode, anchor: &EarAnchor) -> String {
    let mut mode = mode;
    let mut anchor = anchor;

    // Rewrite legacy ear mode to a simpler ear mode to store
    if mode == &EarMode::Behind {
        mode = &EarMode::Out;
        anchor = &EarAnchor::Back;
    }

    let mode_str = match mode {
        EarMode::None => "none",
        EarMode::Above => "above",
        EarMode::Sides => "sides",
        EarMode::Behind => "behind",
        EarMode::Around => "around",
        EarMode::Floppy => "floppy",
        EarMode::Cross => "cross",
        EarMode::Out => "out",
        EarMode::Tall => "tall",
        EarMode::TallCross => "tallcross",
    };

    // Floppy mode doesn't have a separate anchor
    if mode == &EarMode::Floppy {
        return mode_str.to_string();
    }

    let anchor_str = match anchor {
        EarAnchor::Center => "center",
        EarAnchor::Front => "front",
        EarAnchor::Back => "back",
    };

    format!("{}-{}", mode_str, anchor_str)
}

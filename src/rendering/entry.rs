use crate::parts::player_model::PlayerModel;
use image::RgbaImage;

pub struct RenderingEntry {
    pub skin: RgbaImage,
    pub model: PlayerModel,
}

impl RenderingEntry {
    pub fn new(skin: RgbaImage, slim_arms: bool) -> RenderingEntry {
        RenderingEntry {
            skin,
            model: match slim_arms {
                true => PlayerModel::Alex,
                false => PlayerModel::Steve,
            },
        }
    }
}

use crate::parts::player_model::PlayerModel;
use image::RgbaImage;
use crate::uv::Rgba16Image;

pub struct RenderingEntry {
    pub skin: Rgba16Image,
    pub model: PlayerModel,
}

impl RenderingEntry {
    pub fn new(skin: Rgba16Image, slim_arms: bool) -> RenderingEntry {
        RenderingEntry {
            skin,
            model: match slim_arms {
                true => PlayerModel::Alex,
                false => PlayerModel::Steve,
            },
        }
    }
}

use image::buffer::ConvertBuffer;
use image::RgbaImage;
use crate::parts::player_model::PlayerModel;
use crate::uv::Rgba16Image;

pub struct RenderingEntry {
    pub skin: Rgba16Image,
    pub model: PlayerModel,
}

impl RenderingEntry {
    pub fn new(mut skin: RgbaImage, slim_arms: bool) -> RenderingEntry {
        // Strip the alpha data from the skin
        ears_rs::utils::alpha::strip_alpha(&mut skin);

        RenderingEntry {
            skin: skin.convert(),
            model: match slim_arms {
                true => PlayerModel::Alex,
                false => PlayerModel::Steve,
            },
        }
    }
}

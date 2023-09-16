use crate::{types::PlayerBodyPartType, model::ArmorMaterial, parts::part::Part};

use super::{PartsProvider, PlayerPartProviderContext};

pub struct EarsPlayerPartsProvider;

impl<M: ArmorMaterial> PartsProvider<M> for EarsPlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        todo!()
    }
}
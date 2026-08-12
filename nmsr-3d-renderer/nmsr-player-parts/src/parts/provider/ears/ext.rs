use crate::{model::ArmorMaterial, parts::provider::PlayerPartProviderContext};

pub(crate) trait PlayerPartProviderContextExt<M: ArmorMaterial> {
    fn is_wearing_boots(&self) -> bool;
    fn is_wearing_leggings(&self) -> bool;
}

impl<M: ArmorMaterial> PlayerPartProviderContextExt<M> for PlayerPartProviderContext<M> {
    fn is_wearing_boots(&self) -> bool {
        self.armor_slots.as_ref().is_some_and(|s| s.boots.is_some())
    }

    fn is_wearing_leggings(&self) -> bool {
        self.armor_slots
            .as_ref()
            .is_some_and(|s| s.leggings.is_some())
    }
}

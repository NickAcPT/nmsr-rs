use ears_rs::features::{EarsFeatures, data::ear::{EarMode, EarAnchor}};

use crate::{parts::{provider::{PartsProvider, ears::providers::{ears::EarsModEarsPartProvider, builder::EarsModPartBuilder}}, part::Part}, model::ArmorMaterial, types::{PlayerBodyPartType, PlayerPartTextureType}};

use super::PlayerPartProviderContext;

pub(crate) mod ext;
pub(crate) mod providers;

#[derive(Debug, Copy, Clone, Default)]
pub struct EarsPlayerPartsProvider;

impl<M: ArmorMaterial> PartsProvider<M> for EarsPlayerPartsProvider {
    fn get_parts(
        &self,
        context: &PlayerPartProviderContext<M>,
        body_part: PlayerBodyPartType,
    ) -> Vec<Part> {
        let empty = Vec::with_capacity(0);
        
        let provider = EarsModEarsPartProvider::<M>::default();
        
        let Some(mut features) = context.ears_features.filter(|f| provider.provides_for_part(body_part) && provider.provides_for_feature(f, context)) else {
            return empty;
        };
        
        // Replace Behind mode with Out mode w/ Back anchor
        if features.ear_mode == EarMode::Behind {
            features.ear_mode = EarMode::Out;
            features.ear_anchor = EarAnchor::Back;        
        }
        
        let mut parts = Vec::new();
        
        let mut builder = EarsModPartBuilder::new(&mut parts, &context);
        
        provider.provide_parts(&features, context, &mut builder);
        
        parts
    }
}

trait EarsModPartProvider<M: ArmorMaterial> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool;
    
    fn provides_for_feature(&self, feature: &EarsFeatures, context: &PlayerPartProviderContext<M>) -> bool;
    
    fn provide_parts(&self, feature: &EarsFeatures, context: &PlayerPartProviderContext<M>, builder: &mut EarsModPartBuilder<'_, M>);
}


// TODO : Move this to a more appropriate place
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerPartEarsTextureType {
    Cape,
    Wings,
    Emissive,
}

impl PlayerPartEarsTextureType {
    pub fn size(&self) -> (u32, u32) {
        match self {
            Self::Cape | Self::Wings => (20, 16),
            Self::Emissive => (64, 64),
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Cape => "ears_cape",
            Self::Wings => "ears_wings",
            Self::Emissive => "ears_emissive",
        }
    }
}

impl From<PlayerPartEarsTextureType> for PlayerPartTextureType {
    fn from(value: PlayerPartEarsTextureType) -> Self {
        match value {
            PlayerPartEarsTextureType::Cape => PlayerPartTextureType::Cape,
            ears => PlayerPartTextureType::Custom {
                key: ears.key(),
                size: ears.size(),
            },
        }
    }
}
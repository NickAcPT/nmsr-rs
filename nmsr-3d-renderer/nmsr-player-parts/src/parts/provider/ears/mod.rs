use std::marker::PhantomData;

use ears_rs::features::{
    data::ear::{EarAnchor, EarMode},
    EarsFeatures,
};
use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

use crate::{
    model::ArmorMaterial,
    parts::{
        part::Part,
        provider::{
            ears::providers::{
                builder::EarsModPartBuilder, ears::EarsModEarsPartProvider,
                protrusions::EarsModProtrusionsPartProvider, snouts::EarsModSnoutsPartProvider, tails::EarsModTailsPartProvider, wings::EarsModWingsPartProvider, chest::EarsModChestPartProvider,
            },
            PartsProvider,
        },
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
};

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

        let Some(mut features) = context.ears_features else {
            return empty;
        };

        // Replace Behind mode with Out mode w/ Back anchor
        if features.ear_mode == EarMode::Behind {
            features.ear_mode = EarMode::Out;
            features.ear_anchor = EarAnchor::Back;
        }

        let mut parts = Vec::new();
        let mut builder = EarsModPartBuilder::new(&mut parts, &context);
        builder.stack_group("EarsMod", |builder| {
            for provider in EarsModPartStaticDispatch::iter() {
                if !provider.provides_for_part(body_part)
                    || !provider.provides_for_feature(&features, context)
                {
                    continue;
                }

                provider.provide_parts(&features, context, builder, body_part);
            }
        });

        parts
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter, IntoStaticStr)]
enum EarsModPartStaticDispatch<M: ArmorMaterial> {
    Ears(PhantomData<M>),
    Protrusions(PhantomData<M>),
    Snout(PhantomData<M>),
    Tail(PhantomData<M>),
    Wings(PhantomData<M>),
    Chest(PhantomData<M>),
}

impl<M: ArmorMaterial> EarsModPartProvider<M> for EarsModPartStaticDispatch<M> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool {
        match self {
            Self::Ears(_) => EarsModEarsPartProvider::<M>::default().provides_for_part(body_part),
            Self::Protrusions(_) => {
                EarsModProtrusionsPartProvider::<M>::default().provides_for_part(body_part)
            }
            Self::Snout(_) => {
                EarsModSnoutsPartProvider::<M>::default().provides_for_part(body_part)
            }
            Self::Tail(_) => {
                EarsModTailsPartProvider::<M>::default().provides_for_part(body_part)
            }
            Self::Wings(_) => {
                EarsModWingsPartProvider::<M>::default().provides_for_part(body_part)
            }
            Self::Chest(_) => {
                EarsModChestPartProvider::<M>::default().provides_for_part(body_part)
            }
        }
    }

    fn provides_for_feature(
        &self,
        feature: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
    ) -> bool {
        match self {
            Self::Ears(_) => {
                EarsModEarsPartProvider::<M>::default().provides_for_feature(feature, context)
            }
            Self::Protrusions(_) => EarsModProtrusionsPartProvider::<M>::default()
                .provides_for_feature(feature, context),
            Self::Snout(_) => {
                EarsModSnoutsPartProvider::<M>::default().provides_for_feature(feature, context)
            }
            Self::Tail(_) => {
                EarsModTailsPartProvider::<M>::default().provides_for_feature(feature, context)
            }
            Self::Wings(_) => {
                EarsModWingsPartProvider::<M>::default().provides_for_feature(feature, context)
            }
            Self::Chest(_) => {
                EarsModChestPartProvider::<M>::default().provides_for_feature(feature, context)
            }
        }
    }

    fn provide_parts(
        &self,
        feature: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
        body_part: PlayerBodyPartType,
    ) {
        let name: &'static str = self.into();

        builder.stack_group(name, |builder| match self {
            Self::Ears(_) => EarsModEarsPartProvider::<M>::default()
                .provide_parts(feature, context, builder, body_part),
            Self::Protrusions(_) => EarsModProtrusionsPartProvider::<M>::default()
                .provide_parts(feature, context, builder, body_part),
            Self::Snout(_) => EarsModSnoutsPartProvider::<M>::default()
                .provide_parts(feature, context, builder, body_part),
            Self::Tail(_) => EarsModTailsPartProvider::<M>::default()
                .provide_parts(feature, context, builder, body_part),
            Self::Wings(_) => EarsModWingsPartProvider::<M>::default()
                .provide_parts(feature, context, builder, body_part),
            Self::Chest(_) => EarsModChestPartProvider::<M>::default()
                .provide_parts(feature, context, builder, body_part),
        });
    }
}

trait EarsModPartProvider<M: ArmorMaterial> {
    fn provides_for_part(&self, body_part: PlayerBodyPartType) -> bool;

    fn provides_for_feature(
        &self,
        feature: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
    ) -> bool;

    fn provide_parts(
        &self,
        feature: &EarsFeatures,
        context: &PlayerPartProviderContext<M>,
        builder: &mut EarsModPartBuilder<'_, M>,
        body_part: PlayerBodyPartType,
    );
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

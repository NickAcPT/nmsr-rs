use std::marker::PhantomData;

use ears_rs::features::{
    data::ear::{EarAnchor, EarMode},
    EarsFeatures,
};
use itertools::Itertools;
use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

use crate::{
    model::ArmorMaterial,
    parts::{
        part::Part,
        provider::{
            ears::providers::{
                builder::EarsModPartBuilder, chest::EarsModChestPartProvider,
                ears::EarsModEarsPartProvider, protrusions::EarsModProtrusionsPartProvider,
                snouts::EarsModSnoutsPartProvider, tails::EarsModTailsPartProvider,
                wings::EarsModWingsPartProvider,
            },
            minecraft::MinecraftPlayerPartsProvider,
            PartsProvider,
        },
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
};

#[cfg(feature = "part_tracker")]
use crate::parts::provider::minecraft::get_part_group_name;

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
        let action = |builder: &mut EarsModPartBuilder<M>| {
            for provider in EarsModPartStaticDispatch::iter() {
                if !provider.provides_for_part(body_part)
                    || !provider.provides_for_feature(&features, context)
                {
                    continue;
                }

                provider.provide_parts(&features, context, builder, body_part);
            }
        };

        #[cfg(feature = "part_tracker")]
        {
            builder.stack_group(get_part_group_name(body_part.get_non_layer_part()), action);
        }

        #[cfg(not(feature = "part_tracker"))]
        {
            (action)(&mut builder);
        }

        if features.emissive {
            handle_emissive(&mut parts, context, body_part);
        }

        parts
    }
}

fn handle_emissive<M: ArmorMaterial>(
    parts: &mut Vec<Part>,
    context: &PlayerPartProviderContext<M>,
    body_part: PlayerBodyPartType,
) {
    let wing_texture = PlayerPartEarsTextureType::Wings.into();
    let emissive_parts = parts
        .iter()
        // First, we take the parts we know we can change
        .filter(|p| {
            p.get_texture() == PlayerPartTextureType::Skin || p.get_texture() == wing_texture
        })
        // Then we clone them to take ownership of them (so we can mutate later)
        .cloned()
        // Then we also include the default parts from the player since Ears handles all emissives
        .chain(MinecraftPlayerPartsProvider::default().get_parts(context, body_part))
        // Then finally we collect them into a Vec
        .collect_vec();

    for mut emissive_part in emissive_parts {
        let texture = if emissive_part.get_texture() == PlayerPartTextureType::Skin {
            Some(PlayerPartEarsTextureType::EmissiveSkin)
        } else if emissive_part.get_texture() == wing_texture {
            Some(PlayerPartEarsTextureType::EmissiveWings)
        } else {
            None
        };

        let Some(texture) = texture else { continue };

        emissive_part.set_texture(texture.into());

        parts.push(emissive_part);
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
            Self::Tail(_) => EarsModTailsPartProvider::<M>::default().provides_for_part(body_part),
            Self::Wings(_) => EarsModWingsPartProvider::<M>::default().provides_for_part(body_part),
            Self::Chest(_) => EarsModChestPartProvider::<M>::default().provides_for_part(body_part),
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
    /// The non-emissive remaining part of the skin texture.
    EmissiveProcessedSkin,
    /// The non-emissive remaining part of the wings texture.
    EmissiveProcessedWings,
    /// The emissive skin texture type.
    EmissiveSkin,
    /// The emissive wings texture type.
    EmissiveWings,
}

impl PlayerPartEarsTextureType {
    pub fn size(&self) -> (u32, u32) {
        match self {
            Self::Cape | Self::Wings | Self::EmissiveProcessedWings | Self::EmissiveWings => {
                (20, 16)
            }
            Self::EmissiveSkin | Self::EmissiveProcessedSkin => (64, 64),
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Cape => "ears_cape",
            Self::Wings => "ears_wings",
            Self::EmissiveProcessedSkin => "ears_emissive_processed_skin",
            Self::EmissiveProcessedWings => "ears_emissive_processed_wings",
            Self::EmissiveSkin => "ears_emissive_skin",
            Self::EmissiveWings => "ears_emissive_wings",
        }
    }

    pub fn is_emissive(&self) -> bool {
        matches!(self, Self::EmissiveSkin | Self::EmissiveWings)
    }
}

impl From<PlayerPartEarsTextureType> for PlayerPartTextureType {
    fn from(value: PlayerPartEarsTextureType) -> Self {
        match value {
            PlayerPartEarsTextureType::Cape => PlayerPartTextureType::Cape,
            PlayerPartEarsTextureType::EmissiveProcessedSkin => PlayerPartTextureType::Skin,
            PlayerPartEarsTextureType::EmissiveProcessedWings => {
                PlayerPartEarsTextureType::Wings.into()
            }
            ears => PlayerPartTextureType::Custom {
                key: ears.key(),
                size: ears.size(),
                is_emissive: ears.is_emissive(),
            },
        }
    }
}

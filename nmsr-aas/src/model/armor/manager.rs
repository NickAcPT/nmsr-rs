use std::path::PathBuf;

use ears_rs::utils::upgrade_skin_if_needed;
use hyper::Method;
use image::{GenericImageView, RgbaImage};
use nmsr_rendering::high_level::{
    model::{PlayerArmorSlot, PlayerArmorSlots},
    parts::provider::minecraft::compute_base_part,
};
use strum::IntoEnumIterator;
use tokio::fs;
use tracing::Span;

use crate::{
    error::{ArmorManagerError, ArmorManagerResult, ExplainableExt, Result},
    utils::http_client::NmsrHttpClient,
};

use super::{
    VanillaMinecraftArmorMaterial, VanillaMinecraftArmorMaterialData, VanillaMinecraftArmorTrim,
    VanillaMinecraftArmorTrimData, VanillaMinecraftArmorTrimPalette,
};

pub struct VanillaMinecraftArmorManager {
    client: NmsrHttpClient,
    material_location: PathBuf,
    trims_location: PathBuf,
}

enum VanillaArmorApplicable<'a> {
    Armor(VanillaMinecraftArmorMaterial),
    Trim(
        VanillaMinecraftArmorMaterial,
        &'a VanillaMinecraftArmorTrimData,
    ),
}

impl<'a> VanillaArmorApplicable<'a> {
    fn get_layer_name(&self, slot: PlayerArmorSlot) -> String {
        match self {
            Self::Armor(data) => data.get_layer_name(slot.layer_id(), false),
            Self::Trim(_, trim) => trim.trim.get_layer_name(slot.is_leggings()),
        }
    }

    fn apply_modifications_if_needed(&self, image: &mut RgbaImage) {
        if let Self::Trim(armor_material, VanillaMinecraftArmorTrimData { material, .. }) = self {
            let palette = material
                .get_palette_for_trim_armor_material(*armor_material)
                .get_palette_colors();
            let trim_palette = VanillaMinecraftArmorTrimPalette::get_trim_palette();

            for pixel in image.pixels_mut() {
                if pixel[3] == 0 {
                    continue;
                }

                if let Ok(index) = trim_palette.binary_search(&[pixel[0], pixel[1], pixel[2]]) {
                    let actual_color = palette[index];

                    pixel[0] = actual_color[0];
                    pixel[1] = actual_color[1];
                    pixel[2] = actual_color[2];
                }
            }
        }
    }
}

impl VanillaMinecraftArmorManager {
    pub async fn new(cache_path: PathBuf) -> Result<Self> {
        let armor_location = cache_path.join("armor");

        let material_location = armor_location.join("material");
        let trims_location = armor_location.join("trims");

        fs::create_dir_all(&material_location)
            .await
            .explain("Unable to create armor cache folder".to_string())?;

        fs::create_dir_all(&trims_location)
            .await
            .explain("Unable to create armor cache folder".to_string())?;

        let manager = Self {
            client: NmsrHttpClient::new(20),
            material_location,
            trims_location,
        };

        manager.init().await?;

        Ok(manager)
    }

    fn get_material_file_path(&self, material: VanillaMinecraftArmorMaterial) -> PathBuf {
        self.material_location.join(material.to_string())
    }

    fn get_trim_file_path(&self, trim: VanillaMinecraftArmorTrim) -> PathBuf {
        self.trims_location.join(trim.to_string())
    }

    async fn init(&self) -> Result<()> {
        self.download_materials().await?;
        self.download_trims().await?;

        Ok(())
    }

    async fn download_trims(&self) -> Result<()> {
        for trim in VanillaMinecraftArmorTrim::iter() {
            let trim_path = self.get_trim_file_path(trim);

            fs::create_dir_all(&trim_path).await.explain(format!(
                "Unable to create armor cache folder for trim {trim}"
            ))?;

            let layers = trim.get_layer_names();

            for layer in layers {
                let layer_path = trim_path.join(&layer);

                if !layer_path.exists() {
                    let url = format!(
                        "https://raw.githubusercontent.com/NickAcPT/minecraft-assets/24w11a/assets/minecraft/textures/trims/models/armor/{name}",
                        name = &layer
                    );

                    let bytes = self
                        .client
                        .do_request(&url, Method::GET, &Span::current(), || None)
                        .await?;

                    fs::write(&layer_path, bytes)
                        .await
                        .explain(format!("Unable to write armor cache file for trim {layer}"))?;
                }
            }
        }

        Ok(())
    }
    async fn download_materials(&self) -> Result<()> {
        for material in VanillaMinecraftArmorMaterial::iter() {
            let material_path = self.get_material_file_path(material);
            let material_name = material.to_string().to_lowercase();

            fs::create_dir_all(&material_path).await.explain(format!(
                "Unable to create armor cache folder for material {material}"
            ))?;

            let layers = material.get_layer_names();

            for layer in layers {
                let file_name = format!("{material_name}_layer_{layer}.png");
                let layer_path = material_path.join(&file_name);

                if !layer_path.exists() {
                    let url = format!(
                        "https://raw.githubusercontent.com/InventivetalentDev/minecraft-assets/1.20.1/assets/minecraft/textures/models/armor/{name}",
                        name = &file_name
                    );

                    let bytes = self
                        .client
                        .do_request(&url, Method::GET, &Span::current(), || None)
                        .await?;

                    fs::write(&layer_path, bytes).await.explain(format!(
                        "Unable to write armor cache file for material {material}"
                    ))?;
                }
            }
        }

        Ok(())
    }

    fn get_image_path(
        &self,
        applicable: &VanillaArmorApplicable,
        slot: PlayerArmorSlot,
    ) -> PathBuf {
        let root = match applicable {
            VanillaArmorApplicable::Armor(data) => self.get_material_file_path(*data),
            VanillaArmorApplicable::Trim(_, data) => self.get_trim_file_path(data.trim),
        };

        root.join(applicable.get_layer_name(slot))
    }

    pub async fn create_armor_texture(
        &self,
        slots: &PlayerArmorSlots<VanillaMinecraftArmorMaterialData>,
    ) -> Result<(RgbaImage, Option<RgbaImage>)> {
        let mut output_armor_image = image::RgbaImage::new(64, 64);
        let mut output_armor_two_image = image::RgbaImage::new(64, 64);

        for (data, slot) in slots.get_all_materials_in_slots() {
            let output_image = if slot.is_leggings() {
                &mut output_armor_two_image
            } else {
                &mut output_armor_image
            };

            let mut to_apply = vec![VanillaArmorApplicable::Armor(data.material)];

            to_apply.append(
                &mut data
                    .trims
                    .iter()
                    .map(|trim| VanillaArmorApplicable::Trim(data.material, trim))
                    .collect(),
            );

            for aplicable in to_apply {
                self.apply_parts(&aplicable, slot, output_image).await?;
            }
        }

        Ok((
            output_armor_image,
            Some(output_armor_two_image).filter(|_| slots.leggings.is_some()),
        ))
    }

    async fn apply_parts(
        &self,
        applicable: &VanillaArmorApplicable<'_>,
        slot: PlayerArmorSlot,
        output_image: &mut RgbaImage,
    ) -> ArmorManagerResult<()> {
        let material_path = self.get_image_path(applicable, slot);

        let bytes = fs::read(&material_path)
            .await
            .map_err(|_| ArmorManagerError::MissingArmorTextureError(material_path.clone()))?;

        let mut image = image::load_from_memory(&bytes)
            .map_err(|e| ArmorManagerError::ArmorTextureLoadError(material_path.clone(), e))?
            .into_rgba8();

        applicable.apply_modifications_if_needed(&mut image);

        let image = upgrade_skin_if_needed(image);

        for part_type in PlayerArmorSlots::<()>::get_parts_for_armor_slot(slot) {
            let part = compute_base_part(part_type, false);

            let uvs = part.get_face_uvs();
            let uvs = [uvs.up, uvs.down, uvs.north, uvs.south, uvs.east, uvs.west];

            for uv in uvs {
                let x = uv.top_left.x as u32;
                let y = uv.top_left.y as u32;
                let width = uv.bottom_right.x as u32 - x;
                let height = uv.bottom_right.y as u32 - y;

                let view = image.view(x, y, width, height);

                image::imageops::overlay(output_image, &*view, x as i64, y as i64);
            }
        }

        Ok(())
    }
}

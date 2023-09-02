use std::path::PathBuf;

use ears_rs::utils::legacy_upgrader::upgrade_skin_if_needed;
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

use super::{VanillaMinecraftArmorMaterial, VanillaMinecraftArmorMaterialData};

pub struct VanillaMinecraftArmorManager {
    client: NmsrHttpClient,
    armor_location: PathBuf,
}

impl VanillaMinecraftArmorManager {
    pub async fn new(cache_path: PathBuf) -> Result<Self> {
        let armor_location = cache_path.join("armor");

        fs::create_dir_all(&armor_location)
            .await
            .explain("Unable to create armor cache folder".to_string())?;

        let manager = Self {
            client: NmsrHttpClient::new(20),
            armor_location,
        };

        manager.init().await?;

        Ok(manager)
    }

    fn get_material_file_path(&self, material: VanillaMinecraftArmorMaterial) -> PathBuf {
        self.armor_location.join(material.to_string())
    }

    async fn init(&self) -> Result<()> {
        for material in VanillaMinecraftArmorMaterial::iter() {
            let material_path = self.get_material_file_path(material);
            let material_name = material.to_string().to_lowercase();

            fs::create_dir_all(&material_path).await.explain(format!(
                "Unable to create armor cache folder for material {}",
                material.to_string()
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
                        .do_request(&url, Method::GET, &Span::current())
                        .await?;

                    fs::write(&layer_path, bytes).await.explain(format!(
                        "Unable to write armor cache file for material {}",
                        material
                    ))?;
                }
            }
        }

        Ok(())
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
            
            self.apply_armor_parts(data, slot, output_image).await?;
        }

        output_armor_image.save("owo.png").expect("owo");
        output_armor_two_image.save("owo2.png").expect("owo");

        Ok((output_armor_image, Some(output_armor_two_image).filter(|_| slots.leggings.is_some())))
    }

    async fn apply_armor_parts(
        &self,
        data: &VanillaMinecraftArmorMaterialData,
        slot: PlayerArmorSlot,
        output_image: &mut RgbaImage,
    ) -> ArmorManagerResult<()> {
        let material = data.material;
        let material_path = self
            .get_material_file_path(material)
            .join(material.get_layer_name(slot.layer_id(), false));

        let armor_bytes = fs::read(&material_path)
            .await
            .map_err(|_| ArmorManagerError::MissingArmorTextureError(data.clone()))?;

        let armor_image = image::load_from_memory(&armor_bytes)
            .map_err(|e| ArmorManagerError::ArmorTextureLoadError(data.clone(), e))?
            .into_rgba8();

        let image = upgrade_skin_if_needed(armor_image)
            .ok_or(ArmorManagerError::ArmorTextureUpgradeError)?;

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

                image::imageops::overlay(output_image, &*view, x as i64, y as i64)
            }
        }

        Ok(())
    }
}

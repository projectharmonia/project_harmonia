use bevy::prelude::*;
use bevy_egui::egui::{epaint::WHITE_UV, ImageButton, TextureId, Ui};

use crate::core::asset_metadata::AssetMetadata;

pub(super) struct ObjectsTab<'a> {
    metadata: &'a Assets<AssetMetadata>,
}

impl<'a> ObjectsTab<'a> {
    #[must_use]
    pub(super) fn new(metadata: &'a Assets<AssetMetadata>) -> Self {
        Self { metadata }
    }
}

impl ObjectsTab<'_> {
    pub(super) fn show(self, ui: &mut Ui) {
        ui.group(|ui| {
            for metadata in self
                .metadata
                .iter()
                .map(|(_handle_id, metadata)| metadata)
                .filter_map(AssetMetadata::object)
                .filter(|metadata| metadata.is_placable_in_city())
            {
                if ui
                    .add(
                        ImageButton::new(TextureId::Managed(0), (64.0, 64.0))
                            .uv([WHITE_UV, WHITE_UV]),
                    )
                    .on_hover_text(&metadata.name)
                    .clicked()
                {}
            }
        });
    }
}

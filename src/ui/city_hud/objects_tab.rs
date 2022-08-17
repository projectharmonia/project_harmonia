use bevy::prelude::*;
use bevy_egui::egui::{ImageButton, TextureId, Ui};

use crate::core::{
    asset_metadata::AssetMetadata,
    preview::{PreviewRequested, Previews, PREVIEW_SIZE},
};

pub(super) struct ObjectsTab<'a, 'w, 's> {
    metadata: &'a Assets<AssetMetadata>,
    previews: &'a Previews,
    preview_events: &'a mut EventWriter<'w, 's, PreviewRequested>,
}

impl<'a, 'w, 's> ObjectsTab<'a, 'w, 's> {
    #[must_use]
    pub(super) fn new(
        metadata: &'a Assets<AssetMetadata>,
        previews: &'a Previews,
        preview_events: &'a mut EventWriter<'w, 's, PreviewRequested>,
    ) -> Self {
        Self {
            metadata,
            previews,
            preview_events,
        }
    }
}

impl ObjectsTab<'_, '_, '_> {
    pub(super) fn show(self, ui: &mut Ui) {
        ui.group(|ui| {
            for (handle_id, metadata) in self.metadata.iter() {
                let object = match metadata {
                    AssetMetadata::Object(object) if object.is_placable_in_city() => object,
                    _ => continue,
                };

                let texture_id = self.previews.get(&handle_id).unwrap_or_else(|| {
                    self.preview_events.send(PreviewRequested(handle_id));
                    &TextureId::Managed(0)
                });

                if ui
                    .add(ImageButton::new(
                        *texture_id,
                        (PREVIEW_SIZE as f32, PREVIEW_SIZE as f32),
                    ))
                    .on_hover_text(&object.name)
                    .clicked()
                {}
            }
        });
    }
}

use bevy::{asset::HandleId, prelude::*};
use bevy_egui::egui::{ImageButton, TextureId, Ui};

use crate::core::{
    asset_metadata::{AssetMetadata, MetadataKind},
    object::selected_object::SelectedObject,
    preview::{PreviewPlugin, PreviewRequest, Previews},
};

pub(super) struct ObjectsTab<'a, 'w, 's, 'wc, 'sc> {
    commands: &'a mut Commands<'wc, 'sc>,
    metadata: &'a Assets<AssetMetadata>,
    previews: &'a Previews,
    preview_events: &'a mut EventWriter<'w, 's, PreviewRequest>,
    selected_id: Option<HandleId>,
}

impl<'a, 'w, 's, 'wc, 'sc> ObjectsTab<'a, 'w, 's, 'wc, 'sc> {
    #[must_use]
    pub(super) fn new(
        commands: &'a mut Commands<'wc, 'sc>,
        metadata: &'a Assets<AssetMetadata>,
        previews: &'a Previews,
        preview_events: &'a mut EventWriter<'w, 's, PreviewRequest>,
        selected_id: Option<HandleId>,
    ) -> Self {
        Self {
            commands,
            metadata,
            previews,
            preview_events,
            selected_id,
        }
    }
}

impl ObjectsTab<'_, '_, '_, '_, '_> {
    pub(super) fn show(self, ui: &mut Ui) {
        ui.group(|ui| {
            for (id, metadata) in self.metadata.iter().filter(|(_, metadata)| matches!(&metadata.kind, MetadataKind::Object(object) if object.category.is_placable_in_city())) {
                let texture_id = self.previews.get(&id).unwrap_or_else(|| {
                    self.preview_events.send(PreviewRequest(id));
                    &TextureId::Managed(0)
                });

                if ui
                    .add(
                        ImageButton::new(*texture_id, (PreviewPlugin::PREVIEW_SIZE as f32, PreviewPlugin::PREVIEW_SIZE as f32))
                            .selected(matches!(self.selected_id, Some(object) if object == id)),
                    )
                    .on_hover_text(&metadata.general.name)
                    .clicked()
                {
                    self.commands.insert_resource(SelectedObject(id))
                }
            }
        });
    }
}

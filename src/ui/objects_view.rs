use bevy::{asset::HandleId, prelude::*};
use bevy_egui::egui::{ImageButton, TextureId, Ui};

use crate::core::{
    asset_metadata::{AssetMetadata, MetadataKind, ObjectCategory},
    object::selected_object::SelectedObject,
    preview::{PreviewPlugin, PreviewRequest, Previews},
};

pub(super) struct ObjectsView<'a, 'w, 's, 'wc, 'sc> {
    current_category: &'a mut Option<ObjectCategory>,
    categories: &'a [ObjectCategory],
    commands: &'a mut Commands<'wc, 'sc>,
    metadata: &'a Assets<AssetMetadata>,
    previews: &'a Previews,
    preview_events: &'a mut EventWriter<'w, 's, PreviewRequest>,
    selected_id: Option<HandleId>,
}

impl<'a, 'w, 's, 'wc, 'sc> ObjectsView<'a, 'w, 's, 'wc, 'sc> {
    #[must_use]
    pub(super) fn new(
        current_category: &'a mut Option<ObjectCategory>,
        categories: &'a [ObjectCategory],
        commands: &'a mut Commands<'wc, 'sc>,
        metadata: &'a Assets<AssetMetadata>,
        previews: &'a Previews,
        preview_events: &'a mut EventWriter<'w, 's, PreviewRequest>,
        selected_id: Option<HandleId>,
    ) -> Self {
        Self {
            current_category,
            categories,
            commands,
            metadata,
            previews,
            preview_events,
            selected_id,
        }
    }
}

impl ObjectsView<'_, '_, '_, '_, '_> {
    pub(super) fn show(self, ui: &mut Ui) {
        ui.vertical(|ui| {
            if ui.selectable_label(self.current_category.is_none(), "🔠").on_hover_text("All objects").clicked() {
                *self.current_category = None;
            }
            for &category in self.categories {
                if ui.selectable_label(matches!(self.current_category, Some(current_category) if *current_category == category), category.glyph())
                    .on_hover_text(category.to_string()).clicked() {
                        *self.current_category = Some(category);
                    }
            }
        });
        ui.group(|ui| {
            for (id, name) in self
                .metadata
                .iter()
                .filter_map(|(id, metadata)| {
                    if let MetadataKind::Object(object) = &metadata.kind {
                        Some((id, &metadata.general.name, object.category))
                    } else {
                        None
                    }
                })
                .filter(|(.., category)| {
                    if let Some(current_category) = self.current_category {
                        current_category == category
                    } else {
                        self.categories.contains(category)
                    }
                })
                .map(|(id, name, _)| (id, name))
            {
                let texture_id = self.previews.get(&id).unwrap_or_else(|| {
                    self.preview_events.send(PreviewRequest(id));
                    &TextureId::Managed(0)
                });

                const SIZE: (f32, f32) = (
                    PreviewPlugin::PREVIEW_SIZE as f32,
                    PreviewPlugin::PREVIEW_SIZE as f32,
                );
                if ui
                    .add(ImageButton::new(*texture_id, SIZE).selected(
                        matches!(self.selected_id, Some(selected_id) if selected_id == id),
                    ))
                    .on_hover_text(name)
                    .clicked()
                {
                    self.commands.insert_resource(SelectedObject(id))
                }
            }
        });
    }
}

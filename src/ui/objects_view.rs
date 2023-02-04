use bevy::prelude::*;
use bevy_egui::egui::{ImageButton, TextureId, Ui};
use derive_more::Constructor;

use crate::core::{
    asset_metadata::{ObjectCategory, ObjectMetadata},
    object::{placing_object::SpawningObjectBundle, ObjectPath},
    preview::{PreviewPlugin, PreviewRequest, Previews},
};

#[derive(Constructor)]
pub(super) struct ObjectsView<'a, 'w, 's, 'wc, 'sc> {
    current_category: &'a mut Option<ObjectCategory>,
    categories: &'a [ObjectCategory],
    commands: &'a mut Commands<'wc, 'sc>,
    asset_server: &'a AssetServer,
    object_metadata: &'a Assets<ObjectMetadata>,
    previews: &'a Previews,
    preview_events: &'a mut EventWriter<'w, 's, PreviewRequest>,
    selected_path: Option<&'a ObjectPath>,
    spawn_parent: Entity,
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
            for (id, metadata) in self.object_metadata.iter().filter(|(_, metadata)| {
                if let Some(current_category) = self.current_category {
                    *current_category == metadata.category
                } else {
                    self.categories.contains(&metadata.category)
                }
            }) {
                let texture_id = self.previews.get(&id).unwrap_or_else(|| {
                    self.preview_events.send(PreviewRequest(id));
                    &TextureId::Managed(0)
                });
                let asset_path = self
                    .asset_server
                    .get_handle_path(id)
                    .expect("all objects should have a path");

                const SIZE: (f32, f32) = (
                    PreviewPlugin::PREVIEW_SIZE as f32,
                    PreviewPlugin::PREVIEW_SIZE as f32,
                );
                if ui
                    .add(ImageButton::new(*texture_id, SIZE).selected(
                        matches!(self.selected_path, Some(selected_path) if selected_path.0 == asset_path.path()),
                    ))
                    .on_hover_text(&metadata.general.name)
                    .clicked()
                {
                    self.commands
                        .entity(self.spawn_parent)
                        .with_children(|parent| {
                            parent.spawn(SpawningObjectBundle::new(asset_path.path().to_path_buf()));
                        });
                }
            }
        });
    }
}

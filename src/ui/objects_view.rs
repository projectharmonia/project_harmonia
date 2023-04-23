use bevy::{asset::HandleId, prelude::*};
use bevy_egui::egui::{ImageButton, Ui};
use derive_more::Constructor;

use crate::core::{
    asset_metadata::{ObjectCategory, ObjectMetadata},
    object::placing_object::PlacingObject,
    preview::{Preview, PreviewKind, Previews},
};

#[derive(Constructor)]
pub(super) struct ObjectsView<'a, 'w, 's> {
    current_category: &'a mut Option<ObjectCategory>,
    categories: &'a [ObjectCategory],
    commands: &'a mut Commands<'w, 's>,
    object_metadata: &'a Assets<ObjectMetadata>,
    previews: &'a mut Previews,
    selected_id: Option<HandleId>,
    spawn_parent: Entity,
}

impl ObjectsView<'_, '_, '_> {
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
                const ICON_SIZE: f32 = 64.0;
                let texture_id = self.previews.get(Preview {
                    kind: PreviewKind::Object(id),
                    size: ICON_SIZE as u32,
                });
                if ui
                    .add(
                        ImageButton::new(texture_id, (ICON_SIZE, ICON_SIZE)).selected(
                            matches!(self.selected_id, Some(selected_id) if selected_id == id),
                        ),
                    )
                    .on_hover_text(&metadata.general.name)
                    .clicked()
                {
                    self.commands
                        .entity(self.spawn_parent)
                        .with_children(|parent| {
                            parent.spawn(PlacingObject::spawning(id));
                        });
                }
            }
        });
    }
}

use std::iter;

use bevy::prelude::*;

use crate::game_world::{family::FamilyMode, WorldState};

pub(super) struct AlphaColorPlugin;

impl Plugin for AlphaColorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            Self::update_materials
                .run_if(in_state(WorldState::City).or_else(in_state(FamilyMode::Building))),
        );
    }
}

impl AlphaColorPlugin {
    pub(super) fn update_materials(
        mut materials: ResMut<Assets<StandardMaterial>>,
        ghosts: Query<(Entity, &AlphaColor), Changed<AlphaColor>>,
        children: Query<&Children>,
        mut material_handles: Query<&mut Handle<StandardMaterial>>,
    ) {
        let Ok((entity, &alpha)) = ghosts.get_single() else {
            return;
        };

        debug!("setting alpha to `{alpha:?}`");
        let mut iter = material_handles
            .iter_many_mut(iter::once(entity).chain(children.iter_descendants(entity)));
        while let Some(mut material_handle) = iter.fetch_next() {
            let material = materials
                .get(&*material_handle)
                .expect("material handle should be valid");

            // If color matches, assume that we don't need any update.
            if material.base_color == *alpha {
                return;
            }

            let mut material = material.clone();
            material.base_color = *alpha;
            material.alpha_mode = AlphaMode::Add;
            *material_handle = materials.add(material);
        }
    }
}

/// Blends material texture with the given color.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut)]
pub(super) struct AlphaColor(pub(super) Color);

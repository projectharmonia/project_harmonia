use std::iter;

use bevy::{prelude::*, scene::SceneInstanceReady};

use crate::game_world::{family::FamilyMode, WorldState};

pub(super) struct AlphaColorPlugin;

impl Plugin for AlphaColorPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(init_scene).add_systems(
            PostUpdate,
            update_materials.run_if(in_state(WorldState::City).or(in_state(FamilyMode::Building))),
        );
    }
}

fn init_scene(
    trigger: Trigger<SceneInstanceReady>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    alpha_entities: Query<(Entity, &AlphaColor)>,
    children: Query<&Children>,
    mut material_handles: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    if let Ok((entity, &alpha_color)) = alpha_entities.get(trigger.entity()) {
        apply_alpha_color(
            &mut materials,
            &mut material_handles,
            &children,
            entity,
            *alpha_color,
        );
    }
}

pub(super) fn update_materials(
    mut materials: ResMut<Assets<StandardMaterial>>,
    alpha_entities: Query<(Entity, &AlphaColor), Changed<AlphaColor>>,
    children: Query<&Children>,
    mut material_handles: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    for (entity, &alpha_color) in &alpha_entities {
        apply_alpha_color(
            &mut materials,
            &mut material_handles,
            &children,
            entity,
            *alpha_color,
        );
    }
}

fn apply_alpha_color(
    materials: &mut Assets<StandardMaterial>,
    material_handles: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    children: &Query<&Children>,
    entity: Entity,
    alpha_color: Color,
) {
    debug!("setting alpha to `{alpha_color:?}`");
    let mut iter =
        material_handles.iter_many_mut(iter::once(entity).chain(children.iter_descendants(entity)));
    while let Some(mut material_handle) = iter.fetch_next() {
        let Some(material) = materials.get(&*material_handle) else {
            // Skip non-loaded, their alpha color will be updated only after full scene loading anyway.
            return;
        };

        // If color matches, assume that we don't need any update.
        if material.base_color == alpha_color {
            return;
        }

        let mut material = material.clone();
        material.base_color = alpha_color;
        material.alpha_mode = AlphaMode::Add;
        *material_handle = materials.add(material).into();
    }
}

/// Blends material texture with the given color.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut)]
pub(super) struct AlphaColor(pub(super) Color);

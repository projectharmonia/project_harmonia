use avian3d::prelude::*;
use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_mod_outline::{InheritOutline, OutlineVolume};

use super::{
    city::CityMode,
    family::{building::BuildingMode, FamilyMode},
    Layer,
};

pub(super) struct HighlightingPlugin;

impl Plugin for HighlightingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Highlighting>()
            .add_observer(Self::show)
            .add_observer(Self::hide)
            .add_observer(Self::init_scene)
            .add_observer(Self::enable)
            .add_observer(Self::disable)
            .add_systems(OnEnter(BuildingMode::Objects), Self::highlight_objects)
            .add_systems(OnEnter(CityMode::Objects), Self::highlight_objects)
            .add_systems(
                OnEnter(FamilyMode::Life),
                Self::highlight_actors_and_objects,
            )
            .add_systems(OnExit(BuildingMode::Objects), Self::disable_highlighting)
            .add_systems(OnExit(CityMode::Objects), Self::disable_highlighting)
            .add_systems(OnExit(FamilyMode::Life), Self::disable_highlighting);
    }
}

impl HighlightingPlugin {
    fn highlight_objects(mut highlighting: ResMut<Highlighting>) {
        highlighting.mask = Layer::Object.into();
        debug!("enabling highlighting for `{:?}`", highlighting.mask);
    }

    fn highlight_actors_and_objects(mut highlighting: ResMut<Highlighting>) {
        highlighting.mask = [Layer::Actor, Layer::Object].into();
        debug!("enabling highlighting for `{:?}`", highlighting.mask);
    }

    fn disable_highlighting(mut highlighting: ResMut<Highlighting>) {
        debug!("disabling highlighting");
        highlighting.mask = LayerMask::NONE;
    }

    /// Initializes scene children with [`InheritOutlineBundle`] to let toggle only top-level entity.
    fn init_scene(
        trigger: Trigger<SceneInstanceReady>,
        mut commands: Commands,
        scenes: Query<(), With<OutlineVolume>>,
        children: Query<&Children>,
    ) {
        if scenes.get(trigger.entity()).is_err() {
            return;
        }

        debug!("initializing outline for scene `{}`", trigger.entity());
        for child_entity in children.iter_descendants(trigger.entity()) {
            commands.entity(child_entity).insert(InheritOutline);
        }
    }

    fn show(
        trigger: Trigger<Pointer<Over>>,
        mut highlighting: ResMut<Highlighting>,
        disabler: Query<(), With<HighlightDisabler>>,
        mut volumes: Query<(&mut OutlineVolume, &CollisionLayers)>,
    ) {
        let Ok((mut outline, layers)) = volumes.get_mut(trigger.entity()) else {
            return;
        };

        if !highlighting.mask.has_all(layers.memberships) {
            debug!(
                "ignoring highlighting for `{}` due to layers mismatch",
                trigger.entity()
            );
            return;
        }

        highlighting.last_hovered = Some(trigger.entity());
        if disabler.is_empty() {
            debug!("showing highlighting for `{}`", trigger.entity());
            outline.visible = true;
        }
    }

    fn hide(
        trigger: Trigger<Pointer<Out>>,
        mut volumes: Query<&mut OutlineVolume>,
        mut highlighting: ResMut<Highlighting>,
    ) {
        let Ok(mut outline) = volumes.get_mut(trigger.entity()) else {
            return;
        };

        highlighting.last_hovered = None;
        if outline.visible {
            debug!("hiding highlighting for `{}`", trigger.entity());
            outline.visible = false;
        }
    }

    fn disable(
        _trigger: Trigger<OnAdd, HighlightDisabler>,
        mut volumes: Query<&mut OutlineVolume>,
        mut highlighting: ResMut<Highlighting>,
    ) {
        if let Some(entity) = highlighting.last_hovered {
            if let Ok(mut outline) = volumes.get_mut(entity) {
                debug!("disabling highlighting for `{entity}`");
                outline.visible = true;
            } else {
                highlighting.last_hovered = None;
            }
        }
    }

    fn enable(
        _trigger: Trigger<OnRemove, HighlightDisabler>,
        mut highlighting: ResMut<Highlighting>,
        mut volumes: Query<&mut OutlineVolume>,
    ) {
        if let Some(entity) = highlighting.last_hovered {
            if let Ok(mut outline) = volumes.get_mut(entity) {
                debug!("enabling highlighting for `{entity}`");
                outline.visible = true;
            } else {
                highlighting.last_hovered = None;
            }
        }
    }
}

pub(super) const HIGHLIGHTING_VOLUME: OutlineVolume = OutlineVolume {
    visible: false,
    colour: Color::srgba(1.0, 1.0, 1.0, 0.3),
    width: 3.0,
};

#[derive(Resource)]
struct Highlighting {
    mask: LayerMask,
    last_hovered: Option<Entity>,
}

impl Default for Highlighting {
    fn default() -> Self {
        Self {
            mask: LayerMask::NONE,
            last_hovered: None,
        }
    }
}

/// Highlighting will be disabled if any entity with this component is present.
#[derive(Component, Default)]
pub(super) struct HighlightDisabler;

use bevy::prelude::*;
use bevy_trait_query::RegisterExt;
use iyes_loopless::prelude::IntoConditionalSystem;
use serde::{Deserialize, Serialize};

use super::{Task, TaskList, TaskRequestKind};
use crate::core::{game_state::GameState, ground::Ground};

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Task, Walk>()
            .add_system(Self::task_list_system.run_in_state(GameState::Family));
    }
}

impl MovementPlugin {
    fn task_list_system(mut ground: Query<&mut TaskList, (With<Ground>, Added<TaskList>)>) {
        if let Ok(mut task_list) = ground.get_single_mut() {
            task_list.tasks.push(TaskRequestKind::Walk);
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Deserialize, Reflect, Serialize)]
pub(crate) struct Walk(pub(crate) Vec3);

impl Task for Walk {
    fn kind(&self) -> TaskRequestKind {
        TaskRequestKind::Walk
    }
}

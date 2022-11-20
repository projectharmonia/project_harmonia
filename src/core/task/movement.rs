use bevy::prelude::*;
use iyes_loopless::prelude::IntoConditionalSystem;
use serde::{Deserialize, Serialize};

use super::TaskList;
use crate::core::{game_state::GameState, ground::Ground};

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::task_list_system.run_in_state(GameState::Family));
    }
}

impl MovementPlugin {
    fn task_list_system(mut ground: Query<&mut TaskList, (With<Ground>, Added<TaskList>)>) {
        if let Ok(mut task_list) = ground.get_single_mut() {
            let position = task_list.position;
            task_list.tasks.push(Walk(position).into());
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Reflect, Serialize)]
pub(crate) struct Walk(Vec3);

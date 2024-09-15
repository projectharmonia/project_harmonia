use bevy::prelude::*;

pub fn in_any_state<S: States, const SIZE: usize>(
    states: [S; SIZE],
) -> impl FnMut(Option<Res<State<S>>>) -> bool + Clone {
    move |current_state: Option<Res<State<S>>>| match current_state {
        Some(current_state) => states.iter().any(|state| *current_state == *state),
        None => false,
    }
}

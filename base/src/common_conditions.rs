use bevy::prelude::*;

/// Like [`in_state`], but checks for multiple states.
pub fn in_any_state<S: States, const SIZE: usize>(
    states: [S; SIZE],
) -> impl FnMut(Option<Res<State<S>>>) -> bool + Clone {
    move |current_state: Option<Res<State<S>>>| match current_state {
        Some(current_state) => states.iter().any(|state| *current_state == *state),
        None => false,
    }
}

/// Like [`in_state`], but for use inside observers.
pub fn observer_in_state<S: States>(current_state: Option<Res<State<S>>>, state: S) -> bool {
    let Some(current_state) = current_state else {
        return false;
    };

    **current_state == state
}

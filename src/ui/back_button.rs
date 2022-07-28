use bevy_egui::egui::{Align2, Area, Context, PointerButton, Response};
use leafwing_input_manager::prelude::*;

use super::{ui_action::UiAction, UI_MARGIN};

pub(super) struct BackButton<'a> {
    action_state: &'a mut ActionState<UiAction>,
}

impl<'a> BackButton<'a> {
    #[must_use]
    pub(super) fn new(action_state: &'a mut ActionState<UiAction>) -> Self {
        Self { action_state }
    }

    #[must_use]
    pub(super) fn show(self, ctx: &Context) -> Response {
        Area::new("Back area")
            .anchor(Align2::LEFT_BOTTOM, (UI_MARGIN, -UI_MARGIN))
            .show(ctx, |ui| {
                let mut response = ui.button("Back");
                if !response.clicked() && self.action_state.just_pressed(UiAction::Back) {
                    // Count action as click
                    self.action_state.consume(UiAction::Back);
                    response.clicked[PointerButton::Primary as usize] = true;
                }
                response
            })
            .inner
    }
}

use bevy_egui::egui::{
    Align2, Area, Color32, Context, InnerResponse, Pos2, Shape, Ui, WidgetText, Window,
};
use leafwing_input_manager::prelude::ActionState;

use super::ui_action::UiAction;

/// A top level [`Window`] that blocks input to all other widgets.
pub(super) struct ModalWindow<'a> {
    open: &'a mut bool,
    action_state: &'a ActionState<UiAction>,
    window: Window<'a>,
}

impl<'a> ModalWindow<'a> {
    #[must_use]
    /// Creates a new modal [`Window`] with the given state and title.
    pub(super) fn new(
        open: &'a mut bool,
        action_state: &'a ActionState<UiAction>,
        title: impl Into<WidgetText>,
    ) -> Self {
        Self {
            open,
            action_state,
            window: Window::new(title),
        }
    }
}

impl ModalWindow<'_> {
    /// Draws gray area and a [`Window`] on top of it.
    ///
    /// `open` will be set to `false` if [`UiAction::Back`] has been pressed or the window has been closed.
    /// See [`Window::open`] for more details.
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<Option<R>>> {
        // Create an area to prevent interation with other widgets
        // and display semi-transparent background
        const BACKGROUND_ALPHA: u8 = 150;
        Area::new("Modal area")
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                let screen = ui.ctx().input().screen_rect();
                ui.painter().add(Shape::rect_filled(
                    screen,
                    0.0,
                    Color32::from_black_alpha(BACKGROUND_ALPHA),
                ));
                ui.allocate_space(screen.size());
            });

        let inner_response = self
            .window
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .open(self.open)
            .show(ctx, |ui| add_contents(ui));

        if let Some(inner_response) = &inner_response {
            ctx.move_to_top(inner_response.response.layer_id);
        }

        if self.action_state.just_pressed(UiAction::Back) {
            *self.open = false;
        }

        inner_response
    }
}

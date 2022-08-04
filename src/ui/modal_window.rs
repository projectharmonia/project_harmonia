use bevy::prelude::*;
use bevy_egui::egui::{
    Align2, Area, Color32, Context, Id, InnerResponse, Pos2, Shape, Ui, WidgetText, Window,
};
use derive_more::From;
use leafwing_input_manager::prelude::ActionState;

use super::ui_action::UiAction;

/// A top level [`Window`] that blocks input to all other widgets.
pub(super) struct ModalWindow<'a> {
    open: &'a mut bool,
    action_state: &'a mut ActionState<UiAction>,
    window: Window<'a>,
}

impl<'a> ModalWindow<'a> {
    #[must_use]
    /// Creates a new modal [`Window`] with the given state and title.
    pub(super) fn new(
        open: &'a mut bool,
        action_state: &'a mut ActionState<UiAction>,
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
        if self.action_state.just_pressed(UiAction::Back) {
            self.action_state.consume(UiAction::Back);
            *self.open = false;
        }

        let inner_response = self
            .window
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .open(self.open)
            .show(ctx, |ui| add_contents(ui));

        if let Some(inner_response) = &inner_response {
            if ctx
                .data()
                .get_temp_mut_or_default::<ModalIds>(Id::null())
                .is_top_level_or_push(inner_response.response.layer_id.id)
            {
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

                ctx.move_to_top(inner_response.response.layer_id);
            }
        }

        if !*self.open {
            ctx.data()
                .get_temp_mut_or_default::<ModalIds>(Id::null())
                .pop();
        }

        inner_response
    }
}

/// Stack of modal widget IDs.
#[derive(Clone, Default, From, Deref, DerefMut)]
struct ModalIds(Vec<Id>);

impl ModalIds {
    /// Returns `true` if a widget ID is a top-level modal dialog,
    /// or registers a new top-level dialog if it hasn't been registered before.
    fn is_top_level_or_push(&mut self, new_id: Id) -> bool {
        if let Some(pos) = self.iter().position(|&id| id == new_id) {
            pos == self.len() - 1
        } else {
            self.push(new_id);
            true
        }
    }
}

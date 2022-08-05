use std::mem;

use bevy::prelude::*;
use bevy_egui::{
    egui::{
        Align2, Area, Color32, Context, Id, InnerResponse, Pos2, Shape, Ui, WidgetText, Window,
    },
    EguiContext,
};
use leafwing_input_manager::prelude::*;

use super::ui_action::UiAction;

pub(super) struct ModalWindowPlugin;

impl Plugin for ModalWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PreUpdate, Self::update_modal_window_ids);
    }
}

impl ModalWindowPlugin {
    fn update_modal_window_ids(mut egui: ResMut<EguiContext>) {
        egui.ctx_mut()
            .data()
            .get_temp_mut_or_default::<ModalIds>(Id::null())
            .next_frame();
    }
}

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
                .register_modal(inner_response.response.layer_id.id)
            {
                if self.action_state.just_pressed(UiAction::Back) {
                    self.action_state.consume(UiAction::Back);
                    *self.open = false;
                }

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

        inner_response
    }
}

/// Stack of modal widget IDs where last ID is the top modal window.
///
/// There is no reliable way to say if a window was closed (it could happen on a state change, for example),
/// so we remember modal window IDs from the previous frame to detect removals.
#[derive(Clone, Default)]
struct ModalIds {
    current_frame: Vec<Id>,
    previous_frame: Vec<Id>,
}

impl ModalIds {
    /// Registers a new top-level dialog and returns `true` if a widget ID is a top-level modal dialog.
    fn register_modal(&mut self, new_id: Id) -> bool {
        self.current_frame.push(new_id);
        if let Some(pos) = self.previous_frame.iter().position(|&id| id == new_id) {
            pos == self.previous_frame.len() - 1
        } else {
            true
        }
    }

    /// Replaces registered IDs from the previous frame with the current one.
    fn next_frame(&mut self) {
        self.previous_frame = mem::take(&mut self.current_frame);
    }
}

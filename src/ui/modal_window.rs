use bevy::prelude::*;
use bevy_egui::{
    egui::{
        Align2, Area, Color32, Context, Id, InnerResponse, LayerId, Pos2, Shape, Ui, WidgetText,
        Window,
    },
    EguiContext,
};
use leafwing_input_manager::prelude::*;
use smallvec::SmallVec;

use super::ui_action::UiAction;

pub(super) struct ModalWindowPlugin;

impl Plugin for ModalWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PostUpdate, Self::modal_area_system);
    }
}

impl ModalWindowPlugin {
    fn modal_area_system(mut egui: ResMut<EguiContext>) {
        let id = match egui
            .ctx_mut()
            .data()
            .get_temp_mut_or_default::<ModalIds>(Id::null())
            .retain_registered()
        {
            Some(id) => id,
            None => return,
        };

        const BACKGROUND_ALPHA: u8 = 150;
        Area::new(id) // Use id of the widget as an identifier to avoid ordering issues.
            .fixed_pos(Pos2::ZERO)
            .show(egui.ctx_mut(), |ui| {
                let screen = ui.ctx().input().screen_rect();
                ui.painter().add(Shape::rect_filled(
                    screen,
                    0.0,
                    Color32::from_black_alpha(BACKGROUND_ALPHA),
                ));
                ui.allocate_space(screen.size());
            });

        egui.ctx_mut().move_to_top(id);
    }
}

/// A top level [`Window`] that blocks input to all other widgets.
pub(super) struct ModalWindow<'open> {
    title: WidgetText,
    open_state: Option<OpenState<'open>>,
}

impl<'open> ModalWindow<'open> {
    #[must_use]
    /// Creates a new modal [`Window`] with the given state and title.
    pub(super) fn new(title: impl Into<WidgetText>) -> Self {
        Self {
            title: title.into(),
            open_state: None,
        }
    }

    pub(super) fn open(
        mut self,
        open: &'open mut bool,
        action_state: &'open mut ActionState<UiAction>,
    ) -> Self {
        self.open_state = Some(OpenState { open, action_state });
        self
    }
}

impl ModalWindow<'_> {
    /// Draws gray area and a [`Window`] on top of it.
    ///
    /// `open` will be set to `false` if [`UiAction::Back`] has been pressed or the window has been closed.
    /// See [`Window::open`] for more details.
    pub fn show<R>(
        mut self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<Option<R>>> {
        let mut window = Window::new(self.title)
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .collapsible(false)
            .resizable(false);

        if let Some(open_state) = &mut self.open_state {
            window = window.open(open_state.open);
        }

        let inner_response = window.show(ctx, |ui| add_contents(ui));

        if let Some(inner_response) = &inner_response {
            let mut data = ctx.data();
            if data
                .get_temp_mut_or_default::<ModalIds>(Id::null())
                .register_modal(inner_response.response.layer_id)
            {
                if let Some(open_state) = self.open_state {
                    if open_state.action_state.just_pressed(UiAction::Back) {
                        open_state.action_state.consume(UiAction::Back);
                        *open_state.open = false;
                    }
                    if data.get_temp::<ModalClosed>(Id::null()).is_some() {
                        data.remove::<ModalClosed>(Id::null());
                        *open_state.open = false;
                    }
                }
            }
        }

        inner_response
    }
}

struct OpenState<'open> {
    open: &'open mut bool,
    action_state: &'open mut ActionState<UiAction>,
}

/// Stack of modal widget IDs where last ID is the top modal window.
///
/// There is no reliable way to say if a window was closed (it could happen on a state change, for example),
/// so we remember modal window IDs from the previous frame to detect removals.
#[derive(Clone, Default)]
pub struct ModalIds {
    /// IDs that was registered in the previous frame.
    ///
    /// Used to track removals. Order is undefined since [`Self::register_modal`] could be called from any system.
    registered_ids: SmallVec<[LayerId; 3]>,

    /// Stack with modal layers, top level layer is the last one.
    ///
    /// Order is preserved between frames.
    stack: SmallVec<[LayerId; 3]>,
}

impl ModalIds {
    /// Registers a new top-level dialog and returns `true` if a layer ID is a top-level modal dialog.
    fn register_modal(&mut self, new_layer: LayerId) -> bool {
        self.registered_ids.push(new_layer);

        if let Some(pos) = self
            .stack
            .iter()
            .position(|&layer| layer.id == new_layer.id)
        {
            pos == self.stack.len() - 1
        } else {
            self.stack.push(new_layer);
            true
        }
    }

    /// Removes IDs from the stack that wasn't registered, clears the register and returns top layer ID if present.
    fn retain_registered(&mut self) -> Option<LayerId> {
        self.stack.retain(|id| self.registered_ids.contains(id));
        self.registered_ids.clear();
        self.stack.last().copied()
    }

    /// Returns `true` if there is not active modal IDs.
    pub(super) fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

pub(super) trait ModalUiExt {
    fn close_modal(&self);
}

impl ModalUiExt for Ui {
    fn close_modal(&self) {
        self.data().insert_temp(Id::null(), ModalClosed);
    }
}

#[derive(Clone, Copy)]
struct ModalClosed;

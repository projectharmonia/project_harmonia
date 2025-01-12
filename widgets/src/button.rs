use bevy::prelude::*;

use crate::theme::Theme;

pub(super) struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::theme)
            .add_observer(Self::theme_image)
            .add_observer(Self::theme_text)
            .add_observer(Self::toggle)
            .add_systems(
                PostUpdate,
                (
                    Self::update_background,
                    (Self::ensure_single_toggle, Self::switch_tabs).chain(),
                ),
            );
    }
}

impl ButtonPlugin {
    fn theme(
        trigger: Trigger<OnAdd, ButtonKind>,
        theme: Res<Theme>,
        mut buttons: Query<(&ButtonKind, &mut Node)>,
    ) {
        let (button_kind, mut node) = buttons.get_mut(trigger.entity()).unwrap();

        node.justify_content = JustifyContent::Center;
        node.align_items = AlignItems::Center;
        match button_kind {
            ButtonKind::Normal => {
                node.width = theme.button.normal.width;
                node.height = theme.button.normal.height;
            }
            ButtonKind::Large => {
                node.width = theme.button.large.width;
                node.height = theme.button.large.height;
            }
            ButtonKind::Symbol => {
                node.width = theme.button.symbol.width;
                node.height = theme.button.symbol.height;
            }
            ButtonKind::Image => {
                node.width = theme.button.image.width;
                node.height = theme.button.image.height;
            }
        }
    }

    // TODO 0.16: Access hierarchy in the main theme trigger.
    fn theme_image(
        trigger: Trigger<OnAdd, Parent>,
        theme: Res<Theme>,
        mut images: Query<(&Parent, &mut Node), With<ImageNode>>,
        buttons: Query<&ButtonKind>,
    ) {
        let Ok((parent, mut node)) = images.get_mut(trigger.entity()) else {
            return;
        };

        let Ok(&button_kind) = buttons.get(**parent) else {
            return;
        };

        if button_kind == ButtonKind::Image {
            node.width = theme.button.image.image_width;
            node.height = theme.button.image.image_height;
        }
    }

    // TODO 0.16: Access hierarchy in the main theme trigger.
    fn theme_text(
        trigger: Trigger<OnAdd, Parent>,
        theme: Res<Theme>,
        mut text: Query<(&Parent, &mut TextFont, &mut TextColor)>,
        buttons: Query<&ButtonKind>,
    ) {
        let Ok((parent, mut font, mut color)) = text.get_mut(trigger.entity()) else {
            return;
        };

        let Ok(button_kind) = buttons.get(**parent) else {
            return;
        };

        let button_theme = match button_kind {
            ButtonKind::Normal => &theme.button.normal,
            ButtonKind::Large => &theme.button.large,
            ButtonKind::Symbol => &theme.button.symbol,
            ButtonKind::Image => return,
        };

        font.font = button_theme.font.clone();
        font.font_size = button_theme.font_size;
        *color = button_theme.color;
    }

    fn toggle(
        trigger: Trigger<Pointer<Click>>,
        mut buttons: Query<(&mut Toggled, Has<ExclusiveButton>)>,
    ) {
        if let Ok((mut toggled, exclusive)) = buttons.get_mut(trigger.entity()) {
            if !exclusive || !**toggled {
                // Exclusive buttons cannot be untoggled.
                **toggled = !**toggled
            }
        }
    }

    fn update_background(
        theme: Res<Theme>,
        mut buttons: Query<
            (&Interaction, &mut BackgroundColor, Option<&Toggled>),
            (Or<(Changed<Interaction>, Changed<Toggled>)>, With<Button>),
        >,
    ) {
        for (&interaction, mut background, toggled) in &mut buttons {
            let toggled = toggled.map(|toggled| toggled.0).unwrap_or_default();
            *background = match (interaction, toggled) {
                (Interaction::Pressed, _) | (Interaction::None, true) => {
                    theme.button.pressed_background
                }
                (Interaction::Hovered, true) => theme.button.hovered_pressed_background,
                (Interaction::Hovered, false) => theme.button.hovered_background,
                (Interaction::None, false) => theme.button.normal_background,
            };
        }
    }

    fn ensure_single_toggle(
        mut query_cache: Local<Vec<Entity>>,
        mut buttons: Query<(Entity, &mut Toggled), With<ExclusiveButton>>,
        siblings: Query<(Option<&Parent>, Option<&Children>)>,
    ) {
        for (entity, toggled) in &mut buttons {
            if toggled.is_changed() && **toggled {
                debug!("detected toggle for `{entity}`");
                query_cache.push(entity);
            }
        }

        for sibling_entity in query_cache
            .drain(..)
            .flat_map(|entity| siblings.iter_siblings(entity))
        {
            if let Ok((_, mut toggled)) = buttons.get_mut(sibling_entity) {
                if **toggled {
                    debug!("untoggling `{sibling_entity}`");
                    **toggled = false;
                }
            }
        }
    }

    fn switch_tabs(
        mut commands: Commands,
        tabs: Query<(&Toggled, &TabContent), Changed<Toggled>>,
        mut tab_nodes: Query<(&mut Node, Option<&mut PreviousDisplay>)>,
    ) {
        for (toggled, &tab_content) in &tabs {
            let (mut style, mut previous_display) = tab_nodes
                .get_mut(*tab_content)
                .expect("tabs should point to nodes");

            if toggled.0 {
                if let Some(previous_display) = previous_display {
                    style.display = **previous_display;
                }
            } else {
                if let Some(previous_display) = &mut previous_display {
                    ***previous_display = style.display;
                } else {
                    commands
                        .entity(*tab_content)
                        .insert(PreviousDisplay(style.display));
                }

                style.display = Display::None;
            };
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
#[require(Button)]
pub enum ButtonKind {
    Normal,
    Large,
    Symbol,
    Image,
}

/// Makes button behave like tab by changing visibility of the stored entity depending on the value of [`Toggled`].
#[derive(Component, Clone, Copy, Deref)]
#[require(ExclusiveButton)]
pub struct TabContent(pub Entity);

/// Makes the button togglable.
#[derive(Component, Default, Deref, DerefMut)]
pub struct Toggled(pub bool);

/// If present, then only one button that belongs to the parent node can be toggled at any given time.
///
/// The user can click on any button to check it, and that button will replace the existing one as the checked button in the parent node.
#[derive(Component, Default)]
#[require(Toggled)]
pub struct ExclusiveButton;

/// Stores previous [`Display`] since last toggle.
#[derive(Component, Deref, DerefMut)]
struct PreviousDisplay(Display);

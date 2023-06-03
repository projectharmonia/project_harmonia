use bevy::prelude::*;

use super::theme::Theme;

pub(super) struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ExclusivePress>()
            .add_system(Self::interaction_system)
            .add_systems((Self::exclusive_press_system, Self::exclusive_unpress_system).chain());
    }
}

impl ButtonPlugin {
    fn interaction_system(
        theme: Res<Theme>,
        mut buttons: Query<
            (&Interaction, &mut BackgroundColor, Option<&Pressed>),
            Or<(Changed<Interaction>, Changed<Pressed>)>,
        >,
    ) {
        for (&interaction, mut color, pressed) in &mut buttons {
            let pressed = pressed.map(|pressed| pressed.0).unwrap_or_default();
            *color = match (interaction, pressed) {
                (Interaction::Clicked, _) | (Interaction::None, true) => {
                    theme.button.pressed_color.into()
                }
                (Interaction::Hovered, true) => theme.button.hovered_pressed_color.into(),
                (Interaction::Hovered, false) => theme.button.hovered_color.into(),
                (Interaction::None, false) => theme.button.normal_color.into(),
            };
        }
    }

    fn exclusive_press_system(
        mut press_events: EventWriter<ExclusivePress>,
        mut buttons: Query<
            (Entity, &Interaction, &mut Pressed),
            (Changed<Interaction>, With<ExclusiveButton>),
        >,
    ) {
        for (entity, &interaction, mut pressed) in &mut buttons {
            if interaction == Interaction::Clicked {
                pressed.0 = true;
                press_events.send(ExclusivePress(entity));
            }
        }
    }

    fn exclusive_unpress_system(
        mut press_events: EventReader<ExclusivePress>,
        children: Query<&Children>,
        parents: Query<&Parent>,
        mut buttons: Query<(Entity, &mut Pressed)>,
    ) {
        for event in &mut press_events {
            let parent = parents
                .get(event.0)
                .expect("exclusive buttons should always have a parent");
            let children = children.get(**parent).unwrap();
            let mut iter = buttons.iter_many_mut(children);
            while let Some((entity, mut pressed)) = iter.fetch_next() {
                if pressed.0 && entity != event.0 {
                    pressed.0 = false;
                    break;
                }
            }
        }
    }
}

/// Makes the button togglable.
///
/// Used in combination with [`ExclusiveButton`].
#[derive(Component)]
pub(super) struct Pressed(pub(crate) bool);

/// If present, then only one button that belongs to the parent node can be pressed at any given time.
///
/// The user can click on any button to check it, and that button will replace the existing one as the checked button in the parent node.
#[derive(Component)]
pub(super) struct ExclusiveButton;

/// An event that triggered when button with [`ExclusiveButton`] is clicked.
///
/// Used to unpress the other checked button.
struct ExclusivePress(Entity);

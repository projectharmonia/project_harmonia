use bevy::{ecs::query::Has, prelude::*, render::texture::DEFAULT_IMAGE_HANDLE};

use super::click::{Click, LastInteraction};
use crate::ui::theme::Theme;

pub(super) struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::text_init_system,
                Self::image_init_system,
                Self::interaction_system,
                Self::text_update_system,
                Self::toggling_system,
            ),
        )
        .add_systems(
            PostUpdate,
            (Self::exclusive_system, Self::tab_switching_system).chain(),
        );
    }
}

impl ButtonPlugin {
    fn text_init_system(
        mut commmands: Commands,
        theme: Res<Theme>,
        buttons: Query<(Entity, &ButtonText, &TextButtonKind), Added<ButtonText>>,
    ) {
        for (entity, text, kind) in &buttons {
            commmands.entity(entity).with_children(|parent| {
                let style = match kind {
                    TextButtonKind::Normal => theme.button.normal_text.clone(),
                    TextButtonKind::Large => theme.button.large_text.clone(),
                    TextButtonKind::Symbol => theme.button.symbol_text.clone(),
                };
                parent.spawn(TextBundle::from_section(text.0.clone(), style));
            });
        }
    }

    fn image_init_system(
        mut commmands: Commands,
        theme: Res<Theme>,
        buttons: Query<(Entity, &Handle<Image>), (Changed<Handle<Image>>, With<Button>)>,
    ) {
        for (entity, image_handle) in &buttons {
            commmands
                .entity(entity)
                .despawn_descendants()
                .with_children(|parent| {
                    parent.spawn(ImageBundle {
                        style: theme.button.image.clone(),
                        image: UiImage {
                            texture: image_handle.clone(),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });
        }
    }

    /// Won't be triggered after spawning because text child will be spawned at the next frame.
    fn text_update_system(
        buttons: Query<(&Children, &ButtonText), Changed<ButtonText>>,
        mut texts: Query<&mut Text>,
    ) {
        for (children, button_text) in &buttons {
            let mut iter = texts.iter_many_mut(children);
            let mut text = iter.fetch_next().expect("button should have child text");
            text.sections[0].value.clone_from(&button_text.0);
        }
    }

    fn interaction_system(
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
                    theme.button.pressed_color.into()
                }
                (Interaction::Hovered, true) => theme.button.hovered_pressed_color.into(),
                (Interaction::Hovered, false) => theme.button.hovered_color.into(),
                (Interaction::None, false) => theme.button.normal_color.into(),
            };
        }
    }

    fn toggling_system(
        mut click_events: EventReader<Click>,
        mut buttons: Query<(&mut Toggled, Has<ExclusiveButton>)>,
    ) {
        for event in &mut click_events {
            if let Ok((mut toggled, exclusive)) = buttons.get_mut(event.0) {
                if exclusive && toggled.0 {
                    // Button is already pressed, if it's exclusive button, do not toggle it.
                    continue;
                }
                toggled.0 = !toggled.0
            }
        }
    }

    fn exclusive_system(
        mut buttons: Query<
            (Entity, &Parent, &mut Toggled),
            (Changed<Toggled>, With<ExclusiveButton>),
        >,
        children: Query<&Children>,
    ) {
        for (toggled_entity, parent) in buttons
            .iter()
            .filter_map(|(entity, parent, toggled)| toggled.0.then_some((entity, **parent)))
            .collect::<Vec<_>>()
        {
            let children = children.get(parent).unwrap();
            for &child_entity in children.iter().filter(|&&entity| entity != toggled_entity) {
                if let Ok(mut toggled) = buttons.get_component_mut::<Toggled>(child_entity) {
                    if toggled.0 {
                        toggled.0 = false;
                    }
                }
            }
        }
    }

    fn tab_switching_system(
        mut commmands: Commands,
        tabs: Query<(&Toggled, &TabContent), Changed<Toggled>>,
        mut tab_nodes: Query<(&mut Style, Option<&mut PreviousDisplay>)>,
    ) {
        for (toggled, tab_content) in &tabs {
            let (mut style, mut previous_display) = tab_nodes
                .get_mut(tab_content.0)
                .expect("tabs should point to nodes with style component");

            if toggled.0 {
                if let Some(previous_display) = previous_display {
                    style.display = previous_display.0;
                }
            } else {
                if let Some(previous_display) = &mut previous_display {
                    previous_display.0 = style.display;
                } else {
                    commmands
                        .entity(tab_content.0)
                        .insert(PreviousDisplay(style.display));
                }

                style.display = Display::None;
            };
        }
    }
}

/// Makes the button togglable.
#[derive(Component)]
pub(crate) struct Toggled(pub(crate) bool);

/// Stores previous [`Display`] since last toggle.
#[derive(Component)]
struct PreviousDisplay(Display);

/// If present, then only one button that belongs to the parent node can be toggled at any given time.
///
/// The user can click on any button to check it, and that button will replace the existing one as the checked button in the parent node.
#[derive(Component)]
pub(crate) struct ExclusiveButton;

/// Makes button behave like tab by changing visibility of the stored entity depending on the value of [`Pressed`].
#[derive(Component)]
pub(crate) struct TabContent(pub(crate) Entity);

#[derive(Component)]
enum TextButtonKind {
    Normal,
    Large,
    Symbol,
}

#[derive(Component)]
pub(crate) struct ButtonText(pub(crate) String);

#[derive(Bundle)]
pub(crate) struct TextButtonBundle {
    kind: TextButtonKind,
    text: ButtonText,
    last_interaction: LastInteraction,
    pub(crate) button_bundle: ButtonBundle,
}

impl TextButtonBundle {
    pub(crate) fn normal(theme: &Theme, text: impl Into<String>) -> Self {
        Self::new(theme, TextButtonKind::Normal, text)
    }

    pub(crate) fn large(theme: &Theme, text: impl Into<String>) -> Self {
        Self::new(theme, TextButtonKind::Large, text)
    }

    pub(crate) fn symbol(theme: &Theme, text: impl Into<String>) -> Self {
        Self::new(theme, TextButtonKind::Symbol, text)
    }

    fn new(theme: &Theme, kind: TextButtonKind, text: impl Into<String>) -> Self {
        let style = match kind {
            TextButtonKind::Normal => theme.button.normal.clone(),
            TextButtonKind::Large => theme.button.large.clone(),
            TextButtonKind::Symbol => theme.button.symbol.clone(),
        };
        Self {
            kind,
            text: ButtonText(text.into()),
            last_interaction: Default::default(),
            button_bundle: ButtonBundle {
                style,
                background_color: theme.button.normal_color.into(),
                ..Default::default()
            },
        }
    }
}

#[derive(Bundle)]
pub(crate) struct ImageButtonBundle {
    image_handle: Handle<Image>,
    last_interaction: LastInteraction,
    button_bundle: ButtonBundle,
}

impl ImageButtonBundle {
    pub(crate) fn placeholder(theme: &Theme) -> Self {
        Self::new(theme, DEFAULT_IMAGE_HANDLE.typed())
    }

    pub(crate) fn new(theme: &Theme, image_handle: Handle<Image>) -> Self {
        Self {
            image_handle,
            last_interaction: Default::default(),
            button_bundle: ButtonBundle {
                style: theme.button.image_button.clone(),
                background_color: theme.button.normal_color.into(),
                ..Default::default()
            },
        }
    }
}

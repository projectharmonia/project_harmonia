use bevy::{prelude::*, render::texture::DEFAULT_IMAGE_HANDLE};

use crate::ui2::theme::Theme;

pub(crate) struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::text_init_system,
            Self::image_init_system,
            Self::interaction_system,
            Self::toggling_system,
            Self::exclusive_system,
            Self::text_update_system,
            Self::tab_switching_system,
        ));
    }
}

impl ButtonPlugin {
    fn text_init_system(
        mut commmands: Commands,
        theme: Res<Theme>,
        buttons: Query<(Entity, &ButtonText, &ButtonKind), Added<ButtonText>>,
    ) {
        for (entity, text, button_kind) in &buttons {
            commmands.entity(entity).with_children(|parent| {
                let style = match button_kind {
                    ButtonKind::Normal => theme.button.normal_text.clone(),
                    ButtonKind::Large => theme.button.large_text.clone(),
                    ButtonKind::Square => theme.button.square_text.clone(),
                };
                parent.spawn(TextBundle::from_section(text.0.clone(), style));
            });
        }
    }

    fn image_init_system(
        mut commmands: Commands,
        theme: Res<Theme>,
        buttons: Query<(Entity, &Handle<Image>), Added<Button>>,
    ) {
        for (entity, image_handle) in &buttons {
            commmands.entity(entity).with_children(|parent| {
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
            (&Interaction, &mut BackgroundColor, Option<&Pressed>),
            (Or<(Changed<Interaction>, Changed<Pressed>)>, With<Button>),
        >,
    ) {
        for (&interaction, mut background, pressed) in &mut buttons {
            let pressed = pressed.map(|pressed| pressed.0).unwrap_or_default();
            *background = match (interaction, pressed) {
                (Interaction::Clicked, _) | (Interaction::None, true) => {
                    theme.button.pressed_color.into()
                }
                (Interaction::Hovered, true) => theme.button.hovered_pressed_color.into(),
                (Interaction::Hovered, false) => theme.button.hovered_color.into(),
                (Interaction::None, false) => theme.button.normal_color.into(),
            };
        }
    }

    fn toggling_system(mut buttons: Query<(&Interaction, &mut Pressed), Changed<Interaction>>) {
        for (&interation, mut pressed) in &mut buttons {
            if interation == Interaction::Clicked {
                pressed.0 = !pressed.0
            }
        }
    }

    fn exclusive_system(
        mut buttons: Query<
            (Entity, &Parent, &mut Pressed),
            (Changed<Pressed>, With<ExclusiveButton>),
        >,
        children: Query<&Children>,
    ) {
        let pressed_entities: Vec<_> = buttons
            .iter()
            .filter_map(|(entity, parent, pressed)| pressed.0.then_some((entity, **parent)))
            .collect();

        for (pressed_entity, parent) in pressed_entities {
            let children = children.get(parent).unwrap();
            for &child_entity in children.iter().filter(|&&entity| entity != pressed_entity) {
                if let Ok(mut pressed) = buttons.get_component_mut::<Pressed>(child_entity) {
                    if pressed.0 {
                        pressed.0 = false;
                    }
                }
            }
        }
    }

    fn tab_switching_system(
        tabs: Query<(&Pressed, &TabContent), Changed<Pressed>>,
        mut tab_nodes: Query<&mut Style>,
    ) {
        for (pressed, tab_content) in &tabs {
            let mut style = tab_nodes
                .get_mut(tab_content.0)
                .expect("tabs should point to nodes with style component");
            style.display = if pressed.0 {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

/// Makes the button togglable.
#[derive(Component)]
pub(crate) struct Pressed(pub(crate) bool);

/// If present, then only one button that belongs to the parent node can be pressed at any given time.
///
/// The user can click on any button to check it, and that button will replace the existing one as the checked button in the parent node.
#[derive(Component)]
pub(crate) struct ExclusiveButton;

/// Makes button behave like tab by changing visibility of the stored entity depending on the value of [`Pressed`].
#[derive(Component)]
pub(crate) struct TabContent(pub(crate) Entity);

#[derive(Component)]
enum ButtonKind {
    Normal,
    Large,
    Square,
}

#[derive(Component)]
pub(crate) struct ButtonText(pub(crate) String);

#[derive(Bundle)]
pub(crate) struct TextButtonBundle {
    button_kind: ButtonKind,
    button_text: ButtonText,

    #[bundle]
    pub(crate) button_bundle: ButtonBundle,
}

impl TextButtonBundle {
    pub(crate) fn normal(theme: &Theme, text: impl Into<String>) -> Self {
        Self::new(theme, ButtonKind::Normal, text)
    }

    pub(crate) fn large(theme: &Theme, text: impl Into<String>) -> Self {
        Self::new(theme, ButtonKind::Large, text)
    }

    pub(crate) fn square(theme: &Theme, text: impl Into<String>) -> Self {
        Self::new(theme, ButtonKind::Square, text)
    }

    fn new(theme: &Theme, button_kind: ButtonKind, text: impl Into<String>) -> Self {
        let style = match button_kind {
            ButtonKind::Normal => theme.button.normal.clone(),
            ButtonKind::Large => theme.button.large.clone(),
            ButtonKind::Square => theme.button.square.clone(),
        };
        Self {
            button_kind,
            button_text: ButtonText(text.into()),
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

    #[bundle]
    button_bundle: ButtonBundle,
}

impl ImageButtonBundle {
    pub(crate) fn placeholder(theme: &Theme) -> Self {
        Self::new(&theme, DEFAULT_IMAGE_HANDLE.typed())
    }

    pub(crate) fn new(theme: &Theme, image_handle: Handle<Image>) -> Self {
        Self {
            image_handle,
            button_bundle: ButtonBundle {
                style: theme.button.image_button.clone(),
                background_color: theme.button.normal_color.into(),
                ..Default::default()
            },
        }
    }
}

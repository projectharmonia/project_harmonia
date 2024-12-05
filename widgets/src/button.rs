use bevy::{ecs::query::Has, prelude::*};

use super::{
    click::{Click, LastInteraction},
    theme::Theme,
};

pub(super) struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::init_text,
                Self::init_images,
                Self::update_colors,
                Self::update_text,
                Self::toggle,
            ),
        )
        .add_systems(
            PostUpdate,
            (Self::ensure_single_toggle, Self::switch_tabs).chain(),
        );
    }
}

impl ButtonPlugin {
    fn init_text(
        mut commands: Commands,
        theme: Res<Theme>,
        buttons: Query<(Entity, &ButtonText, &TextButtonKind), Added<ButtonText>>,
    ) {
        for (entity, text, kind) in &buttons {
            commands.entity(entity).with_children(|parent| {
                let style = match kind {
                    TextButtonKind::Normal => theme.button.normal_text.clone(),
                    TextButtonKind::Large => theme.button.large_text.clone(),
                    TextButtonKind::Symbol => theme.button.symbol_text.clone(),
                };
                parent.spawn(TextBundle::from_section(text.0.clone(), style));
            });
        }
    }

    fn init_images(
        mut commands: Commands,
        theme: Res<Theme>,
        buttons: Query<(Entity, &Handle<Image>), (Changed<Handle<Image>>, With<Button>)>,
    ) {
        for (entity, image_handle) in &buttons {
            // Entity could be despawned in the same frame, so check for existence.
            if let Some(mut entity) = commands.get_entity(entity) {
                entity.despawn_descendants().with_children(|parent| {
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
    }

    /// Won't be triggered after spawning because text child will be spawned at the next frame.
    fn update_text(
        buttons: Query<(&Children, &ButtonText), Changed<ButtonText>>,
        mut texts: Query<&mut Text>,
    ) {
        for (children, button_text) in &buttons {
            let mut iter = texts.iter_many_mut(children);
            let mut text = iter.fetch_next().expect("button should have child text");
            text.sections[0].value.clone_from(&button_text.0);
        }
    }

    fn update_colors(
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

    fn toggle(
        mut click_events: EventReader<Click>,
        mut buttons: Query<(&mut Toggled, Has<ExclusiveButton>)>,
    ) {
        for event in click_events.read() {
            if let Ok((mut toggled, exclusive)) = buttons.get_mut(event.0) {
                if exclusive && toggled.0 {
                    // Button is already pressed, if it's exclusive button, do not toggle it.
                    continue;
                }
                toggled.0 = !toggled.0
            }
        }
    }

    fn ensure_single_toggle(
        mut query_cache: Local<Vec<Entity>>,
        mut buttons: Query<(Entity, &Parent, &mut Toggled), With<ExclusiveButton>>,
        children: Query<&Children>,
    ) {
        for (toggled_entity, parent, _) in buttons
            .iter_mut()
            .filter(|(.., toggled)| toggled.is_changed() && toggled.0)
        {
            for &other_entity in children.get(**parent).unwrap() {
                if other_entity != toggled_entity {
                    query_cache.push(other_entity);
                }
            }
        }

        let mut iter = buttons.iter_many_mut(&query_cache);
        while let Some((.., mut toggled)) = iter.fetch_next() {
            if toggled.0 {
                toggled.0 = false
            }
        }

        query_cache.clear();
    }

    fn switch_tabs(
        mut commands: Commands,
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
                    commands
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
pub struct Toggled(pub bool);

/// Stores previous [`Display`] since last toggle.
#[derive(Component)]
struct PreviousDisplay(Display);

/// If present, then only one button that belongs to the parent node can be toggled at any given time.
///
/// The user can click on any button to check it, and that button will replace the existing one as the checked button in the parent node.
#[derive(Component)]
pub struct ExclusiveButton;

/// Makes button behave like tab by changing visibility of the stored entity depending on the value of [`Pressed`].
#[derive(Component, Clone, Copy)]
pub struct TabContent(pub Entity);

#[derive(Component)]
enum TextButtonKind {
    Normal,
    Large,
    Symbol,
}

#[derive(Component)]
pub struct ButtonText(pub String);

#[derive(Bundle)]
pub struct TextButtonBundle {
    kind: TextButtonKind,
    text: ButtonText,
    last_interaction: LastInteraction,
    button_bundle: ButtonBundle,
}

impl TextButtonBundle {
    pub fn normal(theme: &Theme, text: impl Into<String>) -> Self {
        Self::new(theme, TextButtonKind::Normal, text)
    }

    pub fn large(theme: &Theme, text: impl Into<String>) -> Self {
        Self::new(theme, TextButtonKind::Large, text)
    }

    pub fn symbol(theme: &Theme, text: impl Into<String>) -> Self {
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

    pub fn with_display(mut self, display: Display) -> Self {
        self.button_bundle.style.display = display;
        self
    }
}

#[derive(Bundle)]
pub struct ImageButtonBundle {
    image_handle: Handle<Image>,
    last_interaction: LastInteraction,
    button_bundle: ButtonBundle,
}

impl ImageButtonBundle {
    pub fn placeholder(theme: &Theme) -> Self {
        Self::new(theme, Default::default())
    }

    pub fn new(theme: &Theme, image_handle: Handle<Image>) -> Self {
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

use bevy::prelude::*;

const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

pub(super) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Theme>()
            .insert_resource(ClearColor(BACKGROUND_COLOR))
            .add_system(Self::button_system);
    }
}

impl ThemePlugin {
    fn button_system(
        theme: Res<Theme>,
        mut interaction_query: Query<
            (&Interaction, &mut BackgroundColor),
            (Changed<Interaction>, With<Button>),
        >,
    ) {
        for (interaction, mut color) in &mut interaction_query {
            *color = match *interaction {
                Interaction::Clicked => theme.button.pressed_color.into(),
                Interaction::Hovered => theme.button.hovered_color.into(),
                Interaction::None => theme.button.normal_color.into(),
            };
        }
    }
}

#[derive(Resource)]
pub(super) struct Theme {
    pub(super) button: ButtonTheme,
    pub(super) text: TextTheme,
}

impl Theme {
    pub(super) fn large_button(&self) -> ButtonBundle {
        ButtonBundle {
            style: self.button.large.clone(),
            background_color: self.button.normal_color.into(),
            ..Default::default()
        }
    }
}

impl FromWorld for Theme {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self {
            button: ButtonTheme {
                large: Style {
                    size: Size::new(Val::Px(200.0), Val::Px(60.0)),
                    margin: UiRect::all(Val::Px(15.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                normal_color: Color::rgb(0.15, 0.15, 0.15),
                hovered_color: Color::rgb(0.25, 0.25, 0.25),
                pressed_color: Color::rgb(0.35, 0.75, 0.35),
            },
            text: TextTheme {
                large: TextStyle {
                    font: asset_server.load("base/fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            },
        }
    }
}

pub(super) struct ButtonTheme {
    pub(super) large: Style,
    pub(super) normal_color: Color,
    pub(super) hovered_color: Color,
    pub(super) pressed_color: Color,
}

#[derive(Resource)]
pub(super) struct TextTheme {
    pub(super) large: TextStyle,
}

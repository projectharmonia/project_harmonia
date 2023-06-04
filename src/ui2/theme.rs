use bevy::prelude::*;

const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

pub(super) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Theme>()
            .insert_resource(ClearColor(BACKGROUND_COLOR));
    }
}

#[derive(Resource)]
pub(super) struct Theme {
    pub(super) button: ButtonTheme,
    pub(super) text: TextTheme,
    pub(super) checkbox: CheckboxTheme,
    pub(super) tab_content_margin: UiRect,
}

impl FromWorld for Theme {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let font_handle = asset_server.load("base/fonts/FiraSans-Bold.ttf");
        Self {
            button: ButtonTheme {
                normal: Style {
                    size: Size::new(Val::Px(170.0), Val::Px(40.0)),
                    margin: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
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
                hovered_pressed_color: Color::rgb(0.25, 0.65, 0.25),
            },
            text: TextTheme {
                normal: TextStyle {
                    font: font_handle.clone(),
                    font_size: 35.0,
                    color: Color::rgb(0.1, 0.1, 0.1),
                },
                normal_button: TextStyle {
                    font: font_handle.clone(),
                    font_size: 35.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
                large_button: TextStyle {
                    font: font_handle,
                    font_size: 40.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            },
            checkbox: CheckboxTheme {
                node: Style {
                    gap: Size::width(Val::Px(10.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                button: Style {
                    size: Size::all(Val::Px(25.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                tick: Style {
                    size: Size::all(Val::Px(15.0)),
                    ..Default::default()
                },
                tick_color: Color::rgb(0.35, 0.75, 0.35),
            },
            tab_content_margin: UiRect::all(Val::Px(20.0)),
        }
    }
}

pub(super) struct ButtonTheme {
    pub(super) normal: Style,
    pub(super) large: Style,
    pub(super) normal_color: Color,
    pub(super) hovered_color: Color,
    pub(super) pressed_color: Color,
    pub(super) hovered_pressed_color: Color,
}

pub(super) struct TextTheme {
    pub(super) normal: TextStyle,
    pub(super) normal_button: TextStyle,
    pub(super) large_button: TextStyle,
}

pub(super) struct CheckboxTheme {
    pub(super) node: Style,
    pub(super) button: Style,
    pub(super) tick: Style,
    pub(super) tick_color: Color,
}

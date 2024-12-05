use bevy::prelude::*;

pub(super) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Theme>()
            .add_systems(Startup, Self::set_clear_color);
    }
}

impl ThemePlugin {
    fn set_clear_color(mut commands: Commands, theme: Res<Theme>) {
        commands.insert_resource(ClearColor(theme.background_color));
    }
}

#[derive(Resource)]
pub struct Theme {
    pub button: ButtonTheme,
    pub label: LabelTheme,
    pub text_edit: TextEditTheme,
    pub checkbox: CheckboxTheme,
    pub gap: GapTheme,
    pub padding: PaddingTheme,
    pub progress_bar: ProgressBarTheme,
    pub background_color: Color,
    pub modal_color: Color,
    pub panel_color: Color,
    pub popup_color: Color,
}

impl FromWorld for Theme {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let text_handle = asset_server.load("base/fonts/FiraSans-Bold.ttf");
        let symbol_handle = asset_server.load("base/fonts/NotoEmoji-Regular.ttf");
        Self {
            button: ButtonTheme {
                normal: Style {
                    width: Val::Px(160.0),
                    height: Val::Px(35.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                large: Style {
                    width: Val::Px(180.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                symbol: Style {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                image_button: Style {
                    width: Val::Px(55.0),
                    height: Val::Px(55.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                image: Style {
                    width: Val::Px(45.0),
                    height: Val::Px(45.0),
                    ..Default::default()
                },
                normal_text: TextStyle {
                    font: text_handle.clone(),
                    font_size: 25.0,
                    color: Color::srgb(0.9, 0.9, 0.9),
                },
                large_text: TextStyle {
                    font: text_handle.clone(),
                    font_size: 30.0,
                    color: Color::srgb(0.9, 0.9, 0.9),
                },
                symbol_text: TextStyle {
                    font: symbol_handle.clone(),
                    font_size: 25.0,
                    color: Color::srgb(0.9, 0.9, 0.9),
                },
                normal_color: Color::srgb(0.15, 0.15, 0.15),
                hovered_color: Color::srgb(0.25, 0.25, 0.25),
                pressed_color: Color::srgb(0.35, 0.75, 0.35),
                hovered_pressed_color: Color::srgb(0.25, 0.65, 0.25),
            },
            label: LabelTheme {
                small: TextStyle {
                    font: text_handle.clone(),
                    font_size: 17.0,
                    color: Color::srgb(0.1, 0.1, 0.1),
                },
                normal: TextStyle {
                    font: text_handle.clone(),
                    font_size: 25.0,
                    color: Color::srgb(0.1, 0.1, 0.1),
                },
                large: TextStyle {
                    font: text_handle.clone(),
                    font_size: 35.0,
                    color: Color::srgb(0.1, 0.1, 0.1),
                },
                symbol: TextStyle {
                    font: symbol_handle,
                    font_size: 20.0,
                    color: Color::srgb(0.1, 0.1, 0.1),
                },
            },
            text_edit: TextEditTheme {
                style: Style {
                    min_width: Val::Px(200.0),
                    border: UiRect::all(Val::Px(5.0)),
                    padding: UiRect::all(Val::Px(5.0)),
                    ..Default::default()
                },
                text: TextStyle {
                    font: text_handle,
                    font_size: 25.0,
                    color: Color::srgb(0.9, 0.9, 0.9),
                },
                background_color: Color::srgb(0.15, 0.15, 0.15),
                active_border: Color::srgb(0.35, 0.75, 0.35),
                inactive_border: Color::srgb(0.35, 0.35, 0.35),
            },
            checkbox: CheckboxTheme {
                node: Style {
                    column_gap: Val::Px(10.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                button: Style {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                tick: Style {
                    width: Val::Px(14.0),
                    height: Val::Px(14.0),
                    ..Default::default()
                },
                tick_color: Color::srgb(0.35, 0.75, 0.35),
            },
            gap: GapTheme {
                normal: Val::Px(10.0),
                large: Val::Px(20.0),
            },
            padding: PaddingTheme {
                normal: UiRect::all(Val::Px(8.0)),
                global: UiRect::all(Val::Px(15.0)),
            },
            progress_bar: ProgressBarTheme {
                background_color: Color::srgb(0.5, 0.5, 0.5),
                fill_color: Color::srgb(0.35, 0.75, 0.35),
            },
            background_color: Color::srgb(0.9, 0.9, 0.9),
            modal_color: Color::srgba(0.0, 0.0, 0.0, 0.0), // TODO: Make gray when we will have multiple UI roots.
            panel_color: Color::srgb(0.8, 0.8, 0.8),
            popup_color: Color::srgb(0.75, 0.75, 0.75),
        }
    }
}

pub struct ButtonTheme {
    pub normal: Style,
    pub large: Style,
    pub symbol: Style,
    pub image_button: Style,
    pub image: Style,
    pub normal_text: TextStyle,
    pub large_text: TextStyle,
    pub symbol_text: TextStyle,
    pub normal_color: Color,
    pub hovered_color: Color,
    pub pressed_color: Color,
    pub hovered_pressed_color: Color,
}

pub struct LabelTheme {
    pub small: TextStyle,
    pub normal: TextStyle,
    pub large: TextStyle,
    pub symbol: TextStyle,
}

pub struct TextEditTheme {
    pub style: Style,
    pub text: TextStyle,
    pub background_color: Color,
    pub inactive_border: Color,
    pub active_border: Color,
}

pub struct CheckboxTheme {
    pub node: Style,
    pub button: Style,
    pub tick: Style,
    pub tick_color: Color,
}

pub struct GapTheme {
    pub normal: Val,
    pub large: Val,
}

pub struct PaddingTheme {
    pub normal: UiRect,
    pub global: UiRect,
}

pub struct ProgressBarTheme {
    pub background_color: Color,
    pub fill_color: Color,
}

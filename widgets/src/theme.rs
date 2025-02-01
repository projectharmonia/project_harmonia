use bevy::prelude::*;

pub(super) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Theme>()
            .add_systems(Startup, set_clear_color);
    }
}

fn set_clear_color(mut commands: Commands, theme: Res<Theme>) {
    commands.insert_resource(ClearColor(theme.background_color.0));
}

#[derive(Resource)]
pub struct Theme {
    pub button: ButtonTheme,
    pub label: LabelTheme,
    pub checkbox: CheckboxTheme,
    pub text_edit: TextEditTheme,
    pub progress_bar: ProgressBarTheme,
    pub gap: GapTheme,
    pub padding: PaddingTheme,
    pub modal_background: BackgroundColor,
    pub popup_background: BackgroundColor,
    pub panel_background: BackgroundColor,
    pub background_color: BackgroundColor,
}

impl FromWorld for Theme {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let text_handle = asset_server.load("base/fonts/FiraSans-Bold.ttf");
        let symbol_handle = asset_server.load("base/fonts/NotoEmoji-Regular.ttf");
        Self {
            button: ButtonTheme {
                normal: TextButtonTheme {
                    width: Val::Px(160.0),
                    height: Val::Px(35.0),
                    font: text_handle.clone(),
                    font_size: 20.0,
                    color: Color::srgb(0.9, 0.9, 0.9).into(),
                },
                large: TextButtonTheme {
                    width: Val::Px(180.0),
                    height: Val::Px(50.0),
                    font: text_handle.clone(),
                    font_size: 25.0,
                    color: Color::srgb(0.9, 0.9, 0.9).into(),
                },
                symbol: TextButtonTheme {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    font: symbol_handle.clone(),
                    font_size: 20.0,
                    color: Color::srgb(0.9, 0.9, 0.9).into(),
                },
                image: ImageButtonTheme {
                    width: Val::Px(55.0),
                    height: Val::Px(55.0),
                    image_width: Val::Px(45.0),
                    image_height: Val::Px(45.0),
                },
                normal_background: Color::srgb(0.15, 0.15, 0.15).into(),
                hovered_background: Color::srgb(0.25, 0.25, 0.25).into(),
                pressed_background: Color::srgb(0.35, 0.75, 0.35).into(),
                hovered_pressed_background: Color::srgb(0.25, 0.65, 0.25).into(),
            },
            label: LabelTheme {
                small: LabelTextTheme {
                    font: text_handle.clone(),
                    font_size: 12.0,
                    color: Color::srgb(0.1, 0.1, 0.1).into(),
                },
                normal: LabelTextTheme {
                    font: text_handle.clone(),
                    font_size: 20.0,
                    color: Color::srgb(0.1, 0.1, 0.1).into(),
                },
                large: LabelTextTheme {
                    font: text_handle.clone(),
                    font_size: 30.0,
                    color: Color::srgb(0.1, 0.1, 0.1).into(),
                },
                symbol: LabelTextTheme {
                    font: symbol_handle,
                    font_size: 15.0,
                    color: Color::srgb(0.1, 0.1, 0.1).into(),
                },
            },
            checkbox: CheckboxTheme {
                column_gap: Val::Px(10.0),
                button_width: Val::Px(20.0),
                button_height: Val::Px(20.0),
                tick_width: Val::Px(14.0),
                tick_height: Val::Px(14.0),
                tick_color: Color::srgb(0.35, 0.75, 0.35).into(),
            },
            text_edit: TextEditTheme {
                min_width: Val::Px(200.0),
                border: UiRect::all(Val::Px(5.0)),
                padding: UiRect::all(Val::Px(5.0)),
                font: text_handle,
                font_size: 20.0,
                text_color: Color::srgb(0.9, 0.9, 0.9).into(),
                background_color: Color::srgb(0.15, 0.15, 0.15).into(),
                active_border: Color::srgb(0.35, 0.75, 0.35).into(),
                inactive_border: Color::srgb(0.35, 0.35, 0.35).into(),
            },
            progress_bar: ProgressBarTheme {
                background_color: Color::srgb(0.5, 0.5, 0.5).into(),
                fill_color: Color::srgb(0.35, 0.75, 0.35).into(),
            },
            gap: GapTheme {
                normal: Val::Px(10.0),
                large: Val::Px(20.0),
            },
            padding: PaddingTheme {
                normal: UiRect::all(Val::Px(8.0)),
                global: UiRect::all(Val::Px(15.0)),
            },
            modal_background: Color::srgba(1.0, 1.0, 1.0, 0.3).into(),
            popup_background: Color::srgb(0.75, 0.75, 0.75).into(),
            panel_background: Color::srgb(0.8, 0.8, 0.8).into(),
            background_color: Color::srgb(0.9, 0.9, 0.9).into(),
        }
    }
}

pub struct ButtonTheme {
    pub normal: TextButtonTheme,
    pub large: TextButtonTheme,
    pub symbol: TextButtonTheme,
    pub image: ImageButtonTheme,
    pub normal_background: BackgroundColor,
    pub hovered_background: BackgroundColor,
    pub pressed_background: BackgroundColor,
    pub hovered_pressed_background: BackgroundColor,
}

pub struct TextButtonTheme {
    pub width: Val,
    pub height: Val,
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color: TextColor,
}

pub struct ImageButtonTheme {
    pub width: Val,
    pub height: Val,
    pub image_width: Val,
    pub image_height: Val,
}

pub struct LabelTheme {
    pub small: LabelTextTheme,
    pub normal: LabelTextTheme,
    pub large: LabelTextTheme,
    pub symbol: LabelTextTheme,
}

pub struct LabelTextTheme {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color: TextColor,
}

pub struct CheckboxTheme {
    pub column_gap: Val,
    pub button_width: Val,
    pub button_height: Val,
    pub tick_width: Val,
    pub tick_height: Val,
    pub tick_color: BackgroundColor,
}

pub struct TextEditTheme {
    pub min_width: Val,
    pub border: UiRect,
    pub padding: UiRect,
    pub font: Handle<Font>,
    pub font_size: f32,
    pub text_color: TextColor,
    pub background_color: BackgroundColor,
    pub inactive_border: BorderColor,
    pub active_border: BorderColor,
}

pub struct ProgressBarTheme {
    pub background_color: BackgroundColor,
    pub fill_color: BackgroundColor,
}

pub struct GapTheme {
    pub normal: Val,
    pub large: Val,
}

pub struct PaddingTheme {
    pub normal: UiRect,
    pub global: UiRect,
}

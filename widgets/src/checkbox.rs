use bevy::prelude::*;

use super::theme::Theme;

pub(crate) struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::init)
            .add_observer(Self::toggle)
            .add_observer(Self::theme_text)
            .add_systems(PostUpdate, Self::update_tick);
    }
}

impl CheckboxPlugin {
    fn init(
        trigger: Trigger<OnAdd, Checkbox>,
        mut commands: Commands,
        theme: Res<Theme>,
        mut checkboxes: Query<&mut Node>,
    ) {
        let mut node = checkboxes.get_mut(trigger.entity()).unwrap();
        node.column_gap = theme.checkbox.column_gap;
        node.flex_direction = FlexDirection::Row;
        node.align_items = AlignItems::Center;

        commands.entity(trigger.entity()).with_children(|parent| {
            parent.spawn((
                Button,
                Node {
                    width: theme.checkbox.button_width,
                    height: theme.checkbox.button_height,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
            ));
        });
    }

    // TODO 0.16: Access hierarchy in the main init trigger.
    fn theme_text(
        trigger: Trigger<OnAdd, Parent>,
        theme: Res<Theme>,
        mut text: Query<(&Parent, &mut TextFont, &mut TextColor)>,
        buttons: Query<(), With<Checkbox>>,
    ) {
        let Ok((parent, mut font, mut color)) = text.get_mut(trigger.entity()) else {
            return;
        };

        if buttons.get(**parent).is_err() {
            return;
        };

        font.font = theme.label.normal.font.clone();
        font.font_size = theme.label.normal.font_size;
        *color = theme.label.normal.color;
    }

    fn toggle(trigger: Trigger<Pointer<Click>>, mut checkboxes: Query<&mut Checkbox>) {
        if let Ok(mut checkbox) = checkboxes.get_mut(trigger.entity()) {
            checkbox.0 = !checkbox.0;
        }
    }

    /// Won't be triggered after spawning because button child will be spawned at the next frame.
    fn update_tick(
        mut commands: Commands,
        theme: Res<Theme>,
        checkboxes: Query<(&Children, &Checkbox), Changed<Checkbox>>,
        buttons: Query<Entity, With<Button>>,
    ) {
        for (children, checkbox) in &checkboxes {
            let entity = buttons
                .iter_many(children)
                .next()
                .expect("checkbox should have child button");
            if checkbox.0 {
                commands.entity(entity).with_children(|parent| {
                    parent.spawn((
                        Node {
                            width: theme.checkbox.tick_width,
                            height: theme.checkbox.tick_height,
                            ..Default::default()
                        },
                        theme.checkbox.tick_color,
                    ));
                });
            } else {
                commands.entity(entity).despawn_descendants();
            }
        }
    }
}

#[derive(Component)]
#[require(Node)]
pub struct Checkbox(pub bool);

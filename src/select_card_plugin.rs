use bevy::prelude::*;

use crate::GameState;

pub(crate) struct SelectCardPlugin;

impl Plugin for SelectCardPlugin {
    fn build(&self, app: &mut App) {
        // This stage allows both players to pick a card. The card modifies various abilities of the player.
        app.add_systems(OnEnter(GameState::PickCard), setup);
        app.add_systems(
            Update,
            switch_state_on_space.run_if(in_state(GameState::PickCard)),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::ColumnReverse,
                flex_grow: 0.,
                flex_shrink: 0.,
                flex_basis: Val::Px(100.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "Move Faster".to_string(),
                        TextStyle {
                            font_size: 20.,
                            color: Color::WHITE,
                            ..Default::default()
                        },
                    ),
                    ..Default::default()
                },
                Node::default(),
                Style::default(),
            ));

            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "More Damage".to_string(),
                        TextStyle {
                            font_size: 20.,
                            color: Color::WHITE,
                            ..Default::default()
                        },
                    ),
                    ..Default::default()
                },
                Node::default(),
                Style::default(),
            ));
        });
}

fn switch_state_on_space(
    keyboard_input: Res<Input<KeyCode>>,
    mut state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        state.set(GameState::Round);
    }
}

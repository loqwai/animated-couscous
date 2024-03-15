use bevy::{prelude::*, text::Text2dBounds};

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
    commands.spawn((Text2dBundle {
        text: Text {
            sections: vec![TextSection::new(
                "HI",
                TextStyle {
                    font_size: 20.,
                    color: Color::WHITE,
                    ..default()
                },
            )],
            ..default()
        },
        text_2d_bounds: Text2dBounds {
            size: Vec2::new(100., 100.),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(0., -100., 0.)),
        ..default()
    },));
}

fn switch_state_on_space(
    keyboard_input: Res<Input<KeyCode>>,
    mut state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        state.set(GameState::Round);
    }
}

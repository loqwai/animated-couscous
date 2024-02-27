use bevy::prelude::*;

use crate::GameState;

pub(crate) struct SelectCardPlugin;

impl Plugin for SelectCardPlugin {
    fn build(&self, app: &mut App) {
        // As this plugin is managing the splash screen, it will focus on the state `GameState::Splash`
        app
            // When entering the state, spawn everything needed for this screen
            .add_systems(
                Update,
                switch_state_on_space.run_if(in_state(GameState::PickCard)),
            );
        // While in this state, run the `countdown` system
        // .add_systems(Update, countdown.run_if(in_state(GameState::Splash)))
        // // When exiting the state, despawn everything that was spawned for this screen
        // .add_systems(OnExit(GameState::Splash), despawn_screen::<OnSplashScreen>);
    }
}

fn switch_state_on_space(
    keyboard_input: Res<Input<KeyCode>>,
    mut state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        state.set(GameState::Round);
    }
}

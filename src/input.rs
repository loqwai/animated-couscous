use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    events::{
        PlayerBlockEvent, PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent,
        PlayerShootEvent, PlayerSpawnEvent,
    },
    manage_state::Player,
    AppConfig, GameState,
};

pub(crate) struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerSpawnEvent>()
            .add_event::<PlayerMoveLeftEvent>()
            .add_event::<PlayerMoveRightEvent>()
            .add_event::<PlayerJumpEvent>()
            .add_event::<PlayerShootEvent>()
            .add_event::<PlayerBlockEvent>()
            .add_systems(
                PreUpdate,
                (
                    on_enter_send_player_spawn,
                    on_a_send_player_move_left,
                    on_d_send_player_move_right,
                    on_space_send_player_jump,
                    on_left_click_send_player_shoot_event,
                    on_right_click_send_player_block,
                )
                    .run_if(in_state(GameState::Round)),
            );
    }
}

fn on_enter_send_player_spawn(
    config: Res<AppConfig>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<PlayerSpawnEvent>,
) {
    if keyboard_input.just_pressed(KeyCode::Return) {
        events.send(PlayerSpawnEvent {
            id: uuid::Uuid::new_v4().to_string(),
            client_id: config.client_id.to_string(),
        });
    }
}

fn on_a_send_player_move_left(
    config: Res<AppConfig>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<PlayerMoveLeftEvent>,
) {
    if keyboard_input.pressed(KeyCode::A) {
        events.send(PlayerMoveLeftEvent {
            id: uuid::Uuid::new_v4().to_string(),
            client_id: config.client_id.to_string(),
        });
    }
}

fn on_d_send_player_move_right(
    config: Res<AppConfig>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<PlayerMoveRightEvent>,
) {
    if keyboard_input.pressed(KeyCode::D) {
        events.send(PlayerMoveRightEvent {
            id: uuid::Uuid::new_v4().to_string(),
            client_id: config.client_id.to_string(),
        });
    }
}

fn on_space_send_player_jump(
    config: Res<AppConfig>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<PlayerJumpEvent>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        events.send(PlayerJumpEvent {
            id: uuid::Uuid::new_v4().to_string(),
            client_id: config.client_id.to_string(),
        });
    }
}

fn on_left_click_send_player_shoot_event(
    config: Res<AppConfig>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    events: EventWriter<PlayerShootEvent>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    players: Query<(&Player, &Transform)>,
) {
    on_left_click_send_player_shoot_event_fallible(
        config,
        mouse_button_input,
        events,
        windows,
        cameras,
        players,
    );
}

fn on_left_click_send_player_shoot_event_fallible(
    config: Res<AppConfig>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut events: EventWriter<PlayerShootEvent>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    players: Query<(&Player, &Transform)>,
) -> Option<()> {
    if !mouse_button_input.just_pressed(MouseButton::Left) {
        return None;
    };

    let cursor_position = windows.get_single().ok()?.cursor_position()?;
    let (camera, camera_transform) = cameras.get_single().ok()?;

    let player = players
        .iter()
        .find(|(p, _)| p.client_id == config.client_id)?;

    let relative_cursor_position = camera
        .viewport_to_world(camera_transform, cursor_position)?
        .origin;
    let aim = (relative_cursor_position - player.1.translation)
        .normalize()
        .xy();

    events.send(PlayerShootEvent {
        id: uuid::Uuid::new_v4().to_string(),
        client_id: config.client_id.to_string(),
        aim,
    });

    None
}

fn on_right_click_send_player_block(
    config: Res<AppConfig>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut events: EventWriter<PlayerBlockEvent>,
) {
    if !mouse_button_input.just_pressed(MouseButton::Right) {
        return;
    };

    events.send(PlayerBlockEvent {
        id: uuid::Uuid::new_v4().to_string(),
        client_id: config.client_id.to_string(),
    });
}

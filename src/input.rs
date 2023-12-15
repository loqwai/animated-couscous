use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    events::{
        PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent, PlayerShootEvent,
        PlayerSpawnEvent,
    },
    manage_state::Player,
    AppConfig,
};

pub(crate) struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerSpawnEvent>()
            .add_event::<PlayerMoveLeftEvent>()
            .add_event::<PlayerMoveRightEvent>()
            .add_event::<PlayerJumpEvent>()
            .add_event::<PlayerShootEvent>()
            .add_systems(
                PreUpdate,
                (
                    on_enter_send_player_spawn,
                    on_a_send_player_move_left,
                    on_d_send_player_move_right,
                    on_space_send_player_jump,
                    on_click_send_player_shoot_event,
                ),
            );
    }
}

fn on_enter_send_player_spawn(
    config: Res<AppConfig>,
    keyboard_input: Res<Input<KeyCode>>,
    mut events: EventWriter<PlayerSpawnEvent>,
) {
    if keyboard_input.just_pressed(KeyCode::Return) {
        events.send(PlayerSpawnEvent {
            client_id: config.client_id.to_string(),
        });
    }
}

fn on_a_send_player_move_left(
    config: Res<AppConfig>,
    keyboard_input: Res<Input<KeyCode>>,
    mut events: EventWriter<PlayerMoveLeftEvent>,
) {
    if keyboard_input.pressed(KeyCode::A) {
        events.send(PlayerMoveLeftEvent {
            client_id: config.client_id.to_string(),
        });
    }
}

fn on_d_send_player_move_right(
    config: Res<AppConfig>,
    keyboard_input: Res<Input<KeyCode>>,
    mut events: EventWriter<PlayerMoveRightEvent>,
) {
    if keyboard_input.pressed(KeyCode::D) {
        events.send(PlayerMoveRightEvent {
            client_id: config.client_id.to_string(),
        });
    }
}

fn on_space_send_player_jump(
    config: Res<AppConfig>,
    keyboard_input: Res<Input<KeyCode>>,
    mut events: EventWriter<PlayerJumpEvent>,
) {
    if keyboard_input.pressed(KeyCode::Space) {
        events.send(PlayerJumpEvent {
            client_id: config.client_id.to_string(),
        });
    }
}

fn on_click_send_player_shoot_event(
    config: Res<AppConfig>,
    mouse_button_input: Res<Input<MouseButton>>,
    events: EventWriter<PlayerShootEvent>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    players: Query<(&Player, &Transform)>,
) {
    on_click_send_player_shoot_event_fallible(
        config,
        mouse_button_input,
        events,
        windows,
        cameras,
        players,
    );
}

fn on_click_send_player_shoot_event_fallible(
    config: Res<AppConfig>,
    mouse_button_input: Res<Input<MouseButton>>,
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
        client_id: config.client_id.to_string(),
        aim,
    });

    None
}

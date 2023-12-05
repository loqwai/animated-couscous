#[macro_use]
extern crate derive_error;

mod level;
mod protos;
mod server;

use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::utils::HashSet;
use bevy::window::WindowPlugin;
use bevy::window::{PrimaryWindow, WindowResolution};
use crossbeam_channel::{Receiver, Sender};
use level::PlayerSpawn;
use protos::generated::applesauce::wrapper::Inner;

use protos::generated::applesauce::{self};

const PLAYER_RADIUS: f32 = 50.;

const BULLET_SPEED: f32 = 800.;
const PLAYER_MOVE_SPEED: f32 = 400.;
const FIRE_TIMEOUT: u64 = 500;
const JUMP_AMOUNT: f32 = 500.;
const GRAVITY: f32 = 3000.;
const TERMINAL_VELOCITY: f32 = 1000.;

const WINDOW_WIDTH: f32 = 1000.;
const WINDOW_HEIGHT: f32 = 300.;

fn main() {
    let window_offset: i32 = std::env::var("WINDOW_OFFSET")
        .unwrap_or("0".to_string())
        .parse()
        .unwrap();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                position: WindowPosition::new(IVec2 {
                    x: 0,
                    y: window_offset,
                }),

                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_event::<BroadcastStateEvent>()
        .add_event::<IAmOutOfSyncEvent>()
        .add_event::<PlayerSyncEvent>()
        .add_event::<BlockEvent>()
        .add_event::<JumpEvent>()
        .add_event::<DespawnPlayerEvent>()
        .add_event::<BulletSyncEvent>()
        .add_systems(Startup, (setup, start_local_server, load_level))
        .add_systems(
            PreUpdate,
            (
                // Update state from network events
                read_network_messages_to_events,
                handle_block_events,
                handle_broadcast_state_event,
                handle_bullet_sync_events,
                handle_despawn_player_events,
                handle_jump_events,
                handle_player_sync_events,
            ),
        )
        .add_systems(
            Update,
            (
                // optional debug systems
                // auto_fire,
                // debug_events,
                // Calculate next game state
                move_moveables.before(apply_velocity),
                apply_velocity_gravity.before(apply_velocity),
                apply_velocity,
                bullet_hit_despawns_player_and_bullet,
                bullet_moves_forward_system,
                cleanup_zombies,
                despawn_shield_on_ttl,
                ensure_main_player,
                shield_blocks_bullets,
            ),
        )
        .add_systems(
            PostUpdate,
            (
                despawn_things_that_need_despawning,
                // Write new state to network
                write_i_am_out_of_sync_events_to_network,
                write_keyboard_as_player_to_network,
                write_mouse_left_clicks_as_bullets_to_network,
                write_mouse_right_clicks_as_blocks_to_network,
                write_space_as_jumps_to_network,
            ),
        )
        .run();
}

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Player {
    id: String,
    color: Color,
    fire_timeout: Timer,
}

#[derive(Component)]
struct MainPlayer;

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    velocity: Velocity,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

#[derive(Bundle)]
struct MainPlayerBundle {
    main_player: MainPlayer,
    player_bundle: PlayerBundle,
}

#[derive(Component)]
struct Velocity(Vec3);

#[derive(Component)]
struct MoveLeft;

#[derive(Component)]
struct MoveRight;

#[derive(Component)]
struct Bullet {
    id: String,
    velocity: Vec3,
}

#[derive(Component)]
struct Shield {
    ttl: Timer,
}

#[derive(Component)]
struct Despawn;

#[derive(Bundle)]
struct ShieldBundle {
    shield: Shield,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

#[derive(Bundle)]
struct BulletBundle {
    bullet: Bullet,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

#[derive(Resource)]
struct NetServer {
    tx: Sender<applesauce::Wrapper>,
    rx: Receiver<applesauce::Wrapper>,
}

#[derive(Resource)]
struct DeadList(HashSet<String>);

#[derive(Clone, Copy)]
struct MoveData {
    moving_left: bool,
    moving_right: bool,
}

#[derive(Event)]
struct BlockEvent {
    player_id: String,
}

#[derive(Event)]
struct JumpEvent {
    player_id: String,
}

#[derive(Event)]
struct DespawnPlayerEvent {
    player_id: String,
}

#[derive(Event)]
struct BulletSyncEvent {
    id: String,
    position: Vec3,
    velocity: Vec3,
}

#[derive(Event)]
struct PlayerSyncEvent {
    player_id: String,
    position: Vec3,
    color: Color,
    move_data: MoveData,
}

#[derive(Event)]
struct BroadcastStateEvent;

#[derive(Event)]
struct IAmOutOfSyncEvent;

fn setup(mut commands: Commands) {
    commands.insert_resource(DeadList(HashSet::new()));
    commands.spawn(Camera2dBundle::default());
}

fn start_local_server(mut commands: Commands) {
    let listen_addr = std::env::var("SERVE_ON").unwrap_or("localhost:3191".to_string());
    let connect_addr = std::env::var("CONNECT_TO").unwrap_or("localhost:3191".to_string());

    let (tx, rx) = server::serve(&listen_addr, &connect_addr);

    commands.insert_resource(NetServer { tx, rx });
}

fn load_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    level::load_level(&mut commands, &mut meshes, &mut materials).unwrap();
}

#[allow(dead_code)]
fn auto_fire(
    mut main_players: Query<(&mut Player, &Transform), With<MainPlayer>>,
    server: Res<NetServer>,
    time: Res<Time>,
) {
    let player = main_players.get_single_mut().ok();
    if player.is_none() {
        return;
    }
    let mut player = player.unwrap();
    player.0.fire_timeout.tick(time.delta());

    if !player.0.fire_timeout.finished() {
        return;
    }

    let bullet_position = player.1.translation.xy() + Vec2::new(71.0, 0.);
    let rotation = Quat::from_rotation_z(std::f32::consts::PI);
    let mut transform = player.1.clone().with_rotation(rotation);
    transform.translation = Vec3::new(bullet_position.x, bullet_position.y, 0.1);

    server
        .tx
        .send(
            applesauce::Bullet {
                id: uuid::Uuid::new_v4().to_string(),
                position: applesauce::Vec3::from(transform.translation).into(),
                velocity: applesauce::Vec3::from(Vec2::new(1., 0.) * BULLET_SPEED).into(),
                special_fields: Default::default(),
            }
            .into(),
        )
        .unwrap();

    let bullet_position = player.1.translation.xy() + Vec2::new(-71.0, 0.);
    let rotation = Quat::from_rotation_z(std::f32::consts::PI * -1.);
    let mut transform = player.1.clone().with_rotation(rotation);
    transform.translation = Vec3::new(bullet_position.x, bullet_position.y, 0.1);

    server
        .tx
        .send(
            applesauce::Bullet {
                id: uuid::Uuid::new_v4().to_string(),
                position: applesauce::Vec3::from(transform.translation).into(),
                velocity: applesauce::Vec3::from(Vec2::new(-1., 0.) * BULLET_SPEED).into(),
                special_fields: Default::default(),
            }
            .into(),
        )
        .unwrap();
}

#[allow(dead_code)]
fn debug_events(
    mut broadcast_state_events: EventReader<BroadcastStateEvent>,
    mut i_am_out_of_sync_events: EventReader<IAmOutOfSyncEvent>,
    mut player_sync_events: EventReader<PlayerSyncEvent>,
    mut block_events: EventReader<BlockEvent>,
    mut despawn_player_events: EventReader<DespawnPlayerEvent>,
    mut bullet_sync_events: EventReader<BulletSyncEvent>,
) {
    for _ in broadcast_state_events.read() {
        println!("broadcast_state_event");
    }
    for _ in i_am_out_of_sync_events.read() {
        println!("i_am_out_of_sync_event");
    }
    for _ in player_sync_events.read() {
        println!("player_sync_event");
    }
    for _ in block_events.read() {
        println!("block_event");
    }
    for _ in despawn_player_events.read() {
        println!("despawn_player_event");
    }
    for _ in bullet_sync_events.read() {
        println!("bullet_sync_event");
    }
}

fn read_network_messages_to_events(
    connection: ResMut<NetServer>,
    mut player_spawn_events: EventWriter<PlayerSyncEvent>,
    mut broadcast_state_events: EventWriter<BroadcastStateEvent>,
    mut block_events: EventWriter<BlockEvent>,
    mut despawn_player_events: EventWriter<DespawnPlayerEvent>,
    mut bullet_sync_events: EventWriter<BulletSyncEvent>,
    mut jump_events: EventWriter<JumpEvent>,
) {
    for input in connection.rx.try_iter() {
        match input.inner.unwrap() {
            Inner::Player(e) => {
                player_spawn_events.send(PlayerSyncEvent {
                    player_id: e.id,
                    position: e.position.unwrap().into(),
                    color: e.color.unwrap().into(),
                    move_data: MoveData {
                        moving_left: e.move_data.moving_left,
                        moving_right: e.move_data.moving_right,
                    },
                });
            }
            Inner::OutOfSync(_) => {
                broadcast_state_events.send(BroadcastStateEvent);
            }
            Inner::Block(e) => {
                block_events.send(BlockEvent {
                    player_id: e.player_id,
                });
            }
            Inner::DespawnPlayer(e) => {
                despawn_player_events.send(DespawnPlayerEvent {
                    player_id: e.player_id,
                });
            }
            Inner::Bullet(e) => {
                bullet_sync_events.send(BulletSyncEvent {
                    id: e.id,
                    position: e.position.unwrap().into(),
                    velocity: e.velocity.unwrap().into(),
                });
            }
            Inner::Jump(e) => jump_events.send(JumpEvent {
                player_id: e.player_id,
            }),
        }
    }
}

fn handle_block_events(
    mut commands: Commands,
    mut block_events: EventReader<BlockEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut out_of_sync_events: EventWriter<IAmOutOfSyncEvent>,
    players: Query<(Entity, &Player)>,
) {
    for event in block_events.read() {
        match players.iter().find(|(_, p)| p.id == event.player_id) {
            None => out_of_sync_events.send(IAmOutOfSyncEvent),
            Some((entity, _)) => {
                let shield = commands
                    .spawn(ShieldBundle {
                        shield: Shield {
                            ttl: Timer::new(Duration::from_millis(100), TimerMode::Once),
                        },
                        mesh_bundle: MaterialMesh2dBundle {
                            mesh: meshes.add(shape::Circle::new(60.).into()).into(),
                            material: materials
                                .add(ColorMaterial::from(Color::rgba(1., 1., 1., 0.2))),
                            transform: Transform::from_translation(Vec3::new(0., 0., 0.1)),
                            ..default()
                        },
                    })
                    .id();
                commands.entity(entity).add_child(shield);
            }
        }
    }
}

fn handle_broadcast_state_event(
    server: ResMut<NetServer>,
    players: Query<(&Player, &Transform, Option<&MoveLeft>, Option<&MoveRight>)>,
    bullets: Query<(&Bullet, &Transform)>,
    mut broadcast_state_events: EventReader<BroadcastStateEvent>,
) {
    if broadcast_state_events.is_empty() {
        return;
    }

    broadcast_state_events.clear();

    players
        .iter()
        .for_each(|(player, transform, move_left, move_right)| {
            server
                .tx
                .send(
                    applesauce::Player {
                        id: player.id.clone(),
                        position: applesauce::Vec3::from(transform.translation).into(),
                        color: applesauce::Color::from(player.color).into(),
                        move_data: applesauce::MoveData::from((
                            move_left.is_some(),
                            move_right.is_some(),
                        ))
                        .into(),
                        special_fields: Default::default(),
                    }
                    .into(),
                )
                .unwrap();
        });

    /* uncommenting the follow code causes the app to hang occasionally */
    bullets.iter().for_each(|(bullet, transform)| {
        server
            .tx
            .send(
                applesauce::Bullet {
                    id: bullet.id.clone(),
                    position: applesauce::Vec3::from(transform.translation).into(),
                    velocity: applesauce::Vec3::from(bullet.velocity).into(),
                    special_fields: Default::default(),
                }
                .into(),
            )
            .unwrap();
    });
}

fn handle_bullet_sync_events(
    mut commands: Commands,
    mut events: EventReader<BulletSyncEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut bullets: Query<(&Bullet, &mut Transform)>,
    dead_list: Res<DeadList>,
) {
    for event in events.read() {
        if dead_list.0.contains(&event.id) {
            continue;
        }

        let rotation = Quat::from_rotation_z(event.velocity.y.atan2(event.velocity.x));

        match bullets.iter_mut().find(|(b, _)| b.id == event.id) {
            Some((_, mut transform)) => {
                transform.translation = event.position;
                transform.rotation = rotation;
            }
            None => {
                commands.spawn(BulletBundle {
                    bullet: Bullet {
                        id: uuid::Uuid::new_v4().to_string(),
                        velocity: event.velocity,
                    },
                    mesh_bundle: MaterialMesh2dBundle {
                        mesh: meshes
                            .add(shape::Quad::new(Vec2::new(40., 10.)).into())
                            .into(),
                        material: materials.add(ColorMaterial::from(Color::WHITE)),
                        transform: Transform {
                            translation: event.position,
                            rotation,
                            ..default()
                        },
                        ..default()
                    },
                });
            }
        };
    }
}

fn handle_despawn_player_events(
    mut commands: Commands,
    players: Query<(Entity, &Player)>,
    mut despawn_player_events: EventReader<DespawnPlayerEvent>,
    mut dead_list: ResMut<DeadList>,
) {
    for event in despawn_player_events.read() {
        dead_list.0.insert(event.player_id.clone());

        if let Some((entity, _)) = players.iter().find(|(_, p)| p.id == event.player_id) {
            commands.entity(entity).insert(Despawn);
        }
    }
}

fn handle_jump_events(
    mut jump_events: EventReader<JumpEvent>,
    mut players: Query<(&Player, &mut Velocity)>,
    mut i_am_out_of_sync_events: EventWriter<IAmOutOfSyncEvent>,
) {
    for event in jump_events.read() {
        match players.iter_mut().find(|(p, _)| p.id == event.player_id) {
            None => i_am_out_of_sync_events.send(IAmOutOfSyncEvent),
            Some((_, mut velocity)) => {
                velocity.0.y = JUMP_AMOUNT;
            }
        }
    }
}

fn handle_player_sync_events(
    mut commands: Commands,
    mut events: EventReader<PlayerSyncEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut existing_players: Query<(Entity, &Player, &mut Transform)>,
    dead_list: Res<DeadList>,
) {
    for event in events.read() {
        if dead_list.0.contains(&event.player_id) {
            continue;
        }

        let entity = match existing_players
            .iter_mut()
            .find(|(_, p, _)| p.id == event.player_id)
        {
            Some((entity, _, mut transform)) => {
                transform.translation = event.position;
                entity
            }
            None => commands
                .spawn(PlayerBundle {
                    player: Player {
                        id: event.player_id.clone(),
                        color: event.color,
                        fire_timeout: Timer::new(
                            Duration::from_millis(FIRE_TIMEOUT),
                            TimerMode::Once,
                        ),
                    },
                    velocity: Velocity(Vec3::new(0., 0., 0.)),
                    mesh_bundle: MaterialMesh2dBundle {
                        mesh: meshes.add(shape::Circle::new(PLAYER_RADIUS).into()).into(),
                        material: materials.add(ColorMaterial::from(event.color)),
                        transform: Transform::from_translation(event.position),
                        ..default()
                    },
                })
                .id(),
        };

        match event.move_data.moving_left {
            true => commands.entity(entity).insert(MoveLeft),
            false => commands.entity(entity).remove::<MoveLeft>(),
        };
        match event.move_data.moving_right {
            true => commands.entity(entity).insert(MoveRight),
            false => commands.entity(entity).remove::<MoveRight>(),
        };
    }
}

fn apply_velocity(mut moveables: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in moveables.iter_mut() {
        transform.translation += time.delta_seconds() * velocity.0;
        transform.translation.y = transform.translation.y.max(PLAYER_RADIUS);
    }
}

fn apply_velocity_gravity(mut velocities: Query<&mut Velocity>, time: Res<Time>) {
    for mut velocity in velocities.iter_mut() {
        velocity.0.y -= time.delta_seconds() * GRAVITY;
        velocity.0.y = velocity.0.y.max(-TERMINAL_VELOCITY);
    }
}

fn bullet_hit_despawns_player_and_bullet(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform, &Bullet), With<Bullet>>,
    mut players: Query<(&Player, &Transform), (With<Player>, Without<Shield>)>,
    server: ResMut<NetServer>,
    mut dead_list: ResMut<DeadList>,
) {
    for bullet in bullets.iter() {
        for player in players.iter_mut() {
            if bullet.1.translation.distance(player.1.translation) < PLAYER_RADIUS {
                commands.entity(bullet.0).insert(Despawn);
                dead_list.0.insert(bullet.2.id.clone());

                server
                    .tx
                    .send(applesauce::DespawnPlayer::from(player.0.id.clone()).into())
                    .unwrap();
            }
        }
    }
}

fn bullet_moves_forward_system(mut bullets: Query<(&Bullet, &mut Transform)>, time: Res<Time>) {
    for (bullet, mut transform) in bullets.iter_mut() {
        transform.translation += bullet.velocity * time.delta_seconds();
    }
}

fn cleanup_zombies(
    mut commands: Commands,
    players: Query<(Entity, &Player)>,
    dead_list: Res<DeadList>,
) {
    players.iter().for_each(|(entity, player)| {
        if !dead_list.0.contains(&player.id) {
            return;
        }
        commands.entity(entity).insert(Despawn);
    });
}

fn despawn_shield_on_ttl(
    mut commands: Commands,
    time: Res<Time>,
    mut shields: Query<(Entity, &mut Shield)>,
) {
    for (entity, mut shield) in shields.iter_mut() {
        shield.ttl.tick(time.delta());
        if shield.ttl.finished() {
            commands.entity(entity).insert(Despawn);
        }
    }
}

fn ensure_main_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    main_players: Query<Entity, With<MainPlayer>>,
    player_spawns: Query<&PlayerSpawn>,
    server: Res<NetServer>,
) {
    if main_players.iter().count() == 0 {
        let id = uuid::Uuid::new_v4().to_string();
        let spawn = player_spawns
            .iter()
            .find(|s| s.player_number == 1)
            .expect("Could not find player 1 spawn point");

        commands.spawn(MainPlayerBundle {
            main_player: MainPlayer,
            player_bundle: PlayerBundle {
                player: Player {
                    id: id.clone(),
                    color: spawn.color,
                    fire_timeout: Timer::new(Duration::from_millis(FIRE_TIMEOUT), TimerMode::Once),
                },
                velocity: Velocity(Vec3::new(0., 0., 0.)),
                mesh_bundle: MaterialMesh2dBundle {
                    mesh: meshes.add(shape::Circle::new(PLAYER_RADIUS).into()).into(),
                    material: materials.add(ColorMaterial::from(spawn.color)),
                    transform: Transform::from_translation(spawn.position),
                    ..default()
                },
            },
        });

        server
            .tx
            .send(
                applesauce::Player {
                    id: id.clone(),
                    position: applesauce::Vec3::from(spawn.position).into(),
                    color: applesauce::Color::from(spawn.color).into(),
                    move_data: applesauce::MoveData::from((false, false)).into(),
                    special_fields: Default::default(),
                }
                .into(),
            )
            .unwrap();
    }
}

fn move_moveables(
    mut left_movers: Query<&mut Velocity, (With<MoveLeft>, Without<MoveRight>)>,
    mut right_movers: Query<&mut Velocity, (With<MoveRight>, Without<MoveLeft>)>,
    mut non_movers: Query<&mut Velocity, (Without<MoveLeft>, Without<MoveRight>)>,
) {
    for mut left_mover in left_movers.iter_mut() {
        left_mover.0.x = -PLAYER_MOVE_SPEED;
    }

    for mut right_mover in right_movers.iter_mut() {
        right_mover.0.x = PLAYER_MOVE_SPEED;
    }

    for mut non_mover in non_movers.iter_mut() {
        non_mover.0.x = 0.;
    }
}

fn shield_blocks_bullets(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    shields: Query<&GlobalTransform, With<Shield>>,
) {
    for (bullet, bloc) in bullets.iter() {
        for shield in shields.iter() {
            if bloc.translation.distance(shield.translation()) < 60. {
                commands.entity(bullet).insert(Despawn);
            }
        }
    }
}

fn write_i_am_out_of_sync_events_to_network(
    server: ResMut<NetServer>,
    mut out_of_sync_events: EventReader<IAmOutOfSyncEvent>,
) {
    if out_of_sync_events.is_empty() {
        return;
    }

    for _ in out_of_sync_events.read() {}

    server.tx.send(applesauce::OutOfSync::new().into()).unwrap();
}

fn write_keyboard_as_player_to_network(
    windows: Query<&Window, With<PrimaryWindow>>,
    main_players: Query<(&Transform, &Player, &Handle<ColorMaterial>), With<MainPlayer>>,
    colors: Res<Assets<ColorMaterial>>,
    keyboard_input: Res<Input<KeyCode>>,
    server: Res<NetServer>,

    left_movers: Query<Entity, (With<MoveLeft>, Without<MoveRight>, With<MainPlayer>)>,
    right_movers: Query<Entity, (With<MoveRight>, Without<MoveLeft>, With<MainPlayer>)>,
) {
    write_keyboard_as_player_to_network_fallible(
        windows,
        main_players,
        colors,
        keyboard_input,
        server,
        left_movers,
        right_movers,
    );
}

fn write_keyboard_as_player_to_network_fallible(
    windows: Query<&Window, With<PrimaryWindow>>,
    main_players: Query<(&Transform, &Player, &Handle<ColorMaterial>), With<MainPlayer>>,
    colors: Res<Assets<ColorMaterial>>,
    keyboard_input: Res<Input<KeyCode>>,
    server: Res<NetServer>,

    left_movers: Query<Entity, (With<MoveLeft>, Without<MoveRight>, With<MainPlayer>)>,
    right_movers: Query<Entity, (With<MoveRight>, Without<MoveLeft>, With<MainPlayer>)>,
) -> Option<()> {
    windows.get_single().unwrap().cursor_position()?;

    let player_moving_left = left_movers.get_single().is_ok();
    let player_moving_right = right_movers.get_single().is_ok();

    let (player_transform, player, color_handle) = main_players.get_single().ok()?;
    let color = colors.get(color_handle).unwrap().color;

    let a_pressed = keyboard_input.pressed(KeyCode::A);
    let d_pressed = keyboard_input.pressed(KeyCode::D);

    if a_pressed != player_moving_left || d_pressed != player_moving_right {
        server
            .tx
            .send(
                applesauce::Player {
                    id: player.id.clone(),
                    position: applesauce::Vec3::from(player_transform.translation).into(),
                    color: applesauce::Color::from(color).into(),
                    move_data: applesauce::MoveData::from((a_pressed, d_pressed)).into(),
                    special_fields: Default::default(),
                }
                .into(),
            )
            .unwrap();
    }

    None
}

fn write_mouse_left_clicks_as_bullets_to_network(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    main_players: Query<(&mut Player, &Transform), With<MainPlayer>>,
    server: Res<NetServer>,
    time: Res<Time>,
) {
    write_mouse_left_clicks_as_bullets_to_network_fallible(
        mouse_button_input,
        windows,
        cameras,
        main_players,
        server,
        time,
    );
}

fn write_mouse_left_clicks_as_bullets_to_network_fallible(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut main_players: Query<(&mut Player, &Transform), With<MainPlayer>>,
    server: Res<NetServer>,
    time: Res<Time>,
) -> Option<()> {
    let mut player = main_players.get_single_mut().ok()?;
    player.0.fire_timeout.tick(time.delta());

    if !mouse_button_input.just_pressed(MouseButton::Left) || !player.0.fire_timeout.finished() {
        return None;
    };

    player.0.fire_timeout.reset();

    let cursor_position = windows.get_single().unwrap().cursor_position()?;
    let (camera, camera_transform) = cameras.get_single().unwrap();
    let relative_cursor_position = camera
        .viewport_to_world(camera_transform, cursor_position)
        .unwrap()
        .origin;
    let aim = (relative_cursor_position - player.1.translation)
        .normalize()
        .xy();

    let rotation = Quat::from_rotation_z(aim.y.atan2(aim.x));
    let mut transform = player.1.clone().with_rotation(rotation);

    // offset the bullet so they don't shoot themselves
    let bullet_half_length = 20.;
    let fudge_factor = 1.;
    let offset = PLAYER_RADIUS + bullet_half_length + fudge_factor;
    let bullet_position = transform.translation.xy() + aim.clamp_length_min(offset);

    transform.translation = Vec3::new(bullet_position.x, bullet_position.y, 0.1);

    server
        .tx
        .send(
            applesauce::Bullet {
                id: uuid::Uuid::new_v4().to_string(),
                position: applesauce::Vec3::from(transform.translation).into(),
                velocity: applesauce::Vec3::from(aim.normalize() * BULLET_SPEED).into(),
                special_fields: Default::default(),
            }
            .into(),
        )
        .unwrap();

    Some(())
}

fn write_mouse_right_clicks_as_blocks_to_network(
    mouse_button_input: Res<Input<MouseButton>>,
    server: Res<NetServer>,
    main_players: Query<&Player, With<MainPlayer>>,
) {
    write_mouse_right_clicks_as_blocks_to_network_fallible(
        mouse_button_input,
        server,
        main_players,
    );
}

fn write_mouse_right_clicks_as_blocks_to_network_fallible(
    mouse_button_input: Res<Input<MouseButton>>,
    server: Res<NetServer>,
    main_players: Query<&Player, With<MainPlayer>>,
) -> Option<()> {
    let player = main_players.get_single().ok()?;

    if !mouse_button_input.just_pressed(MouseButton::Right) {
        return None;
    }

    server
        .tx
        .send(
            applesauce::Block {
                player_id: player.id.clone(),
                special_fields: Default::default(),
            }
            .into(),
        )
        .unwrap();

    None
}

fn write_space_as_jumps_to_network(
    keyboard_input: Res<Input<KeyCode>>,
    server: Res<NetServer>,
    main_players: Query<&Player, With<MainPlayer>>,
) {
    write_space_as_jumps_to_network_fallible(keyboard_input, server, main_players);
}

fn write_space_as_jumps_to_network_fallible(
    keyboard_input: Res<Input<KeyCode>>,
    server: Res<NetServer>,
    main_players: Query<&Player, With<MainPlayer>>,
) -> Option<()> {
    if !keyboard_input.pressed(KeyCode::Space) {
        return None;
    }
    let player = main_players.get_single().ok()?;

    server
        .tx
        .send(
            applesauce::Jump {
                player_id: player.id.clone(),
                special_fields: Default::default(),
            }
            .into(),
        )
        .unwrap();

    None
}

fn despawn_things_that_need_despawning(
    mut commands: Commands,
    entities: Query<Entity, With<Despawn>>,
) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

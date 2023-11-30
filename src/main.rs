mod protos;
mod server;

use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::utils::HashSet;
use bevy::window::WindowPlugin;
use bevy::window::{PrimaryWindow, WindowResolution};
use crossbeam_channel::{Receiver, Sender};
use protos::generated::applesauce::wrapper::Inner;
use rand::prelude::*;

use protos::generated::applesauce::{self};

const BULLET_SPEED: f32 = 800.;
const PLAYER_MOVE_SPEED: f32 = 400.;
const FIRE_TIMEOUT: u64 = 500;

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
        .add_event::<DespawnPlayerEvent>()
        .add_event::<BulletSyncEvent>()
        .add_systems(Startup, (setup, start_local_server))
        .add_systems(
            PreUpdate,
            (
                // Update state from network events
                read_network_messages_to_events,
                handle_block_events,
                handle_broadcast_state_event,
                handle_bullet_sync_events,
                handle_despawn_player_events,
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
                bullet_hit_despawns_player_and_bullet,
                bullet_moves_forward_system,
                cleanup_zombies,
                despawn_shield_on_ttl,
                ensure_main_player,
                move_moveables,
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
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

#[derive(Bundle)]
struct MainPlayerBundle {
    main_player: MainPlayer,
    player_bundle: PlayerBundle,
}

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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(DeadList(HashSet::new()));
    commands.spawn(Camera2dBundle::default());

    // Ground
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::Quad::new(Vec2::new(1000., 1000.)).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::GRAY)),
        transform: Transform::from_translation(Vec3::new(0., -500., -0.1)),
        ..default()
    });
}

fn start_local_server(mut commands: Commands) {
    let listen_addr = std::env::var("SERVE_ON").unwrap_or("localhost:3191".to_string());
    let connect_addr = std::env::var("CONNECT_TO").unwrap_or("localhost:3191".to_string());

    let (tx, rx) = server::serve(&listen_addr, &connect_addr);

    commands.insert_resource(NetServer { tx, rx });
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
                    mesh_bundle: MaterialMesh2dBundle {
                        mesh: meshes.add(shape::Circle::new(50.).into()).into(),
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

fn bullet_hit_despawns_player_and_bullet(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform, &Bullet), With<Bullet>>,
    mut players: Query<(&Player, &Transform), (With<Player>, Without<Shield>)>,
    server: ResMut<NetServer>,
    mut dead_list: ResMut<DeadList>,
) {
    for bullet in bullets.iter() {
        for player in players.iter_mut() {
            if bullet.1.translation.distance(player.1.translation) < 50. {
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
    server: Res<NetServer>,
) {
    if main_players.iter().count() == 0 {
        let id = uuid::Uuid::new_v4().to_string();
        let mut rng = rand::thread_rng();
        let mut x: f32 = rng.gen();
        x *= 1000.;
        x -= 500.;
        let z: f32 = rng.gen();

        let r = rng.gen();
        let g = rng.gen();
        let b = rng.gen();

        commands.spawn(MainPlayerBundle {
            main_player: MainPlayer,
            player_bundle: PlayerBundle {
                player: Player {
                    id: id.clone(),
                    color: Color::rgb(r, g, b),
                    fire_timeout: Timer::new(Duration::from_millis(FIRE_TIMEOUT), TimerMode::Once),
                },
                mesh_bundle: MaterialMesh2dBundle {
                    mesh: meshes.add(shape::Circle::new(50.).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::rgb(r, g, b))),
                    transform: Transform::from_translation(Vec3::new(x, 50., z)),
                    ..default()
                },
            },
        });

        server
            .tx
            .send(
                applesauce::Player {
                    id: id.clone(),
                    position: applesauce::Vec3::from(Vec3::new(x, 50., z)).into(),
                    color: applesauce::Color::from(Color::rgb(r, g, b)).into(),
                    move_data: applesauce::MoveData::from((false, false)).into(),
                    special_fields: Default::default(),
                }
                .into(),
            )
            .unwrap();
    }
}

fn move_moveables(
    mut left_movers: Query<&mut Transform, (With<MoveLeft>, Without<MoveRight>)>,
    mut right_movers: Query<&mut Transform, (With<MoveRight>, Without<MoveLeft>)>,
    time: Res<Time>,
) {
    for mut left_mover in left_movers.iter_mut() {
        // move the player left, but compensate for how much time has passed since the last update
        left_mover.translation.x -= PLAYER_MOVE_SPEED * time.delta_seconds();
    }

    for mut right_mover in right_movers.iter_mut() {
        right_mover.translation.x += PLAYER_MOVE_SPEED * time.delta_seconds();
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
) {
    write_keyboard_as_player_to_network_fallible(
        windows,
        main_players,
        colors,
        keyboard_input,
        server,
    );
}

fn write_keyboard_as_player_to_network_fallible(
    windows: Query<&Window, With<PrimaryWindow>>,
    main_players: Query<(&Transform, &Player, &Handle<ColorMaterial>), With<MainPlayer>>,
    colors: Res<Assets<ColorMaterial>>,
    keyboard_input: Res<Input<KeyCode>>,
    server: Res<NetServer>,
) -> Option<()> {
    windows.get_single().unwrap().cursor_position()?;

    let (player_transform, player, color_handle) = main_players.get_single().ok()?;
    let color = colors.get(color_handle).unwrap().color;

    let a_just_pressed = keyboard_input.just_pressed(KeyCode::A);
    let d_just_pressed = keyboard_input.just_pressed(KeyCode::D);
    let a_just_released = keyboard_input.just_released(KeyCode::A);
    let d_just_released = keyboard_input.just_released(KeyCode::D);
    let a_pressed = keyboard_input.pressed(KeyCode::A);
    let d_pressed = keyboard_input.pressed(KeyCode::D);

    if a_just_pressed || d_just_pressed || a_just_released || d_just_released {
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
    let player_radius = 50.;
    let bullet_half_length = 20.;
    let fudge_factor = 1.;
    let offset = player_radius + bullet_half_length + fudge_factor;
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

fn despawn_things_that_need_despawning(
    mut commands: Commands,
    entities: Query<Entity, With<Despawn>>,
) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

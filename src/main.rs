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
use uuid::Uuid;

fn main() {
    let window_offset: i32 = std::env::var("WINDOW_OFFSET")
        .unwrap_or("0".to_string())
        .parse()
        .unwrap();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(1000., 300.),
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
        .add_event::<FireEvent>()
        .add_event::<BlockEvent>()
        .add_event::<DespawnPlayerEvent>()
        .add_systems(Startup, setup)
        .add_systems(Startup, start_local_server)
        .add_systems(Update, ensure_main_player)
        .add_systems(Update, move_moveables)
        .add_systems(Update, fire_bullets)
        .add_systems(Update, bullet_moves_forward_system)
        .add_systems(Update, sync_players)
        .add_systems(Update, bullet_hit_despawns_player)
        .add_systems(Update, write_inputs_to_server)
        .add_systems(Update, incoming_network_messages_to_events)
        .add_systems(Update, broadcast_state)
        .add_systems(Update, broadcast_i_am_out_of_sync)
        .add_systems(Update, activate_shield)
        .add_systems(Update, shield_blocks_bullets)
        .add_systems(Update, despawn_shield_on_ttl)
        .add_systems(Update, despawn_player_on_despawn_player_event)
        .add_systems(PostUpdate, despawn_things_that_need_despawning)
        .run();
}

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Player {
    id: String,
    color: Color,
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
struct Bullet;

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
struct FireEvent {
    player_id: String,
    aim_x: f32,
    aim_y: f32,
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

fn incoming_network_messages_to_events(
    connection: ResMut<NetServer>,
    mut player_spawn_events: EventWriter<PlayerSyncEvent>,
    mut out_of_sync_events: EventWriter<BroadcastStateEvent>,
    mut fire_events: EventWriter<FireEvent>,
    mut block_events: EventWriter<BlockEvent>,
    mut despawn_player_events: EventWriter<DespawnPlayerEvent>,
) {
    for input in connection.rx.try_iter() {
        match input.inner.unwrap() {
            Inner::PlayerSync(e) => {
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
                out_of_sync_events.send(BroadcastStateEvent);
            }
            Inner::Fire(e) => {
                fire_events.send(FireEvent {
                    player_id: e.player_id,
                    aim_x: e.aim_x,
                    aim_y: e.aim_y,
                });
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
        }
    }
}

fn bullet_moves_forward_system(mut bullets: Query<&mut Transform, With<Bullet>>) {
    for mut bullet in bullets.iter_mut() {
        // move bullet forward, taking it's rotation into account
        let rotation = bullet.rotation * Vec3::X * 10.;
        bullet.translation += rotation;
    }
}

fn despawn_things_that_need_despawning(
    mut commands: Commands,
    entities: Query<Entity, With<Despawn>>,
) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn move_moveables(
    mut left_movers: Query<&mut Transform, (With<MoveLeft>, Without<MoveRight>)>,
    mut right_movers: Query<&mut Transform, (With<MoveRight>, Without<MoveLeft>)>,
) {
    for mut left_mover in left_movers.iter_mut() {
        left_mover.translation.x -= 2.;
    }

    for mut right_mover in right_movers.iter_mut() {
        right_mover.translation.x += 2.;
    }
}

fn fire_bullets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut events: EventReader<FireEvent>,
    mut out_of_sync_events: EventWriter<IAmOutOfSyncEvent>,
    players: Query<(&Player, &Transform)>,
) {
    for event in events.read() {
        match players.iter().find(|(p, _)| p.id == event.player_id) {
            Some((_, transform)) => {
                let ray = Vec3::new(event.aim_x, event.aim_y, 0.);
                let rotation = Quat::from_rotation_z(ray.y.atan2(ray.x));
                let mut transform = transform.clone().with_rotation(rotation);

                // offset the bullet so they don't shoot themselves
                let player_radius = 50.;
                let bullet_half_length = 20.;
                let fudge_factor = 1.;
                transform.translation += ray
                    .normalize()
                    .clamp_length_min(player_radius + bullet_half_length + fudge_factor);
                transform.translation.z = 0.1;

                commands.spawn(BulletBundle {
                    bullet: Bullet,
                    mesh_bundle: MaterialMesh2dBundle {
                        mesh: meshes
                            .add(shape::Quad::new(Vec2::new(40., 10.)).into())
                            .into(),
                        material: materials.add(ColorMaterial::from(Color::WHITE)),
                        transform,
                        ..default()
                    },
                });
            }
            None => {
                out_of_sync_events.send(IAmOutOfSyncEvent);
            }
        };
    }
}

fn activate_shield(
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

fn ensure_main_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    main_players: Query<Entity, With<MainPlayer>>,
    mut broadcast_state_events: EventWriter<BroadcastStateEvent>,
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
                    id,
                    color: Color::rgb(r, g, b),
                },
                mesh_bundle: MaterialMesh2dBundle {
                    mesh: meshes.add(shape::Circle::new(50.).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::rgb(r, g, b))),
                    transform: Transform::from_translation(Vec3::new(x, 50., z)),
                    ..default()
                },
            },
        });

        broadcast_state_events.send(BroadcastStateEvent);
    }
}

fn sync_players(
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

fn bullet_hit_despawns_player(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    mut players: Query<(&Player, &Transform), (With<Player>, Without<Shield>)>,
    server: ResMut<NetServer>,
) {
    for bullet in bullets.iter() {
        for player in players.iter_mut() {
            if bullet.1.translation.distance(player.1.translation) < 50. {
                commands.entity(bullet.0).insert(Despawn);

                server
                    .tx
                    .send(applesauce::Wrapper {
                        id: uuid::Uuid::new_v4().to_string(),
                        inner: applesauce::DespawnPlayer::from(player.0.id.clone()).into(),
                        special_fields: Default::default(),
                    })
                    .unwrap();
            }
        }
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

fn despawn_player_on_despawn_player_event(
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

fn write_inputs_to_server(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    main_players: Query<(&Transform, &Player, &Handle<ColorMaterial>), With<MainPlayer>>,
    colors: Res<Assets<ColorMaterial>>,
    keyboard_input: Res<Input<KeyCode>>,
    server: Res<NetServer>,
) {
    write_inputs_to_server_fallible(
        windows,
        cameras,
        mouse_button_input,
        main_players,
        colors,
        keyboard_input,
        server,
    );
}

fn write_inputs_to_server_fallible(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    main_players: Query<(&Transform, &Player, &Handle<ColorMaterial>), With<MainPlayer>>,
    colors: Res<Assets<ColorMaterial>>,
    keyboard_input: Res<Input<KeyCode>>,
    server: Res<NetServer>,
) -> Option<()> {
    windows.get_single().unwrap().cursor_position()?;

    let (camera, camera_transform) = cameras.get_single().unwrap();
    let cursor = windows.get_single().unwrap().cursor_position().unwrap();
    let cursor_position = camera
        .viewport_to_world(camera_transform, cursor)
        .unwrap()
        .origin;

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
            .send(applesauce::Wrapper {
                id: Uuid::new_v4().to_string(),
                inner: Some(Inner::PlayerSync(applesauce::Player {
                    id: player.id.clone(),
                    position: applesauce::Vec3::from(player_transform.translation).into(),
                    color: applesauce::Color::from(color).into(),
                    move_data: applesauce::MoveData::from((a_pressed, d_pressed)).into(),
                    special_fields: Default::default(),
                })),
                special_fields: Default::default(),
            })
            .unwrap();
    }

    if mouse_button_input.just_pressed(MouseButton::Left) {
        let aim_vector = cursor_position - player_transform.translation;
        server
            .tx
            .send(applesauce::Wrapper {
                id: Uuid::new_v4().to_string(),
                inner: Some(Inner::Fire(applesauce::Fire {
                    player_id: player.id.clone(),
                    aim_x: aim_vector.x,
                    aim_y: aim_vector.y,
                    ..Default::default()
                })),
                ..Default::default()
            })
            .unwrap();
    }

    if mouse_button_input.just_pressed(MouseButton::Right) {
        server
            .tx
            .send(applesauce::Wrapper {
                id: Uuid::new_v4().to_string(),
                inner: Some(Inner::Block(applesauce::Block {
                    player_id: player.id.clone(),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .unwrap();
    }

    Some(())
}

fn broadcast_state(
    server: ResMut<NetServer>,
    players: Query<(&Player, &Transform, Option<&MoveLeft>, Option<&MoveRight>)>,
    mut broadcast_state_events: EventReader<BroadcastStateEvent>,
) {
    if broadcast_state_events.is_empty() {
        return;
    }

    for _ in broadcast_state_events.read() {}

    players
        .iter()
        .map(
            |(player, transform, move_left, move_right)| applesauce::Player {
                id: player.id.clone(),
                position: applesauce::Vec3::from(transform.translation).into(),
                color: applesauce::Color::from(player.color).into(),
                move_data: applesauce::MoveData::from((move_left.is_some(), move_right.is_some()))
                    .into(),
                special_fields: Default::default(),
            },
        )
        .for_each(|player| {
            server
                .tx
                .send(applesauce::Wrapper {
                    id: uuid::Uuid::new_v4().to_string(),
                    inner: Some(Inner::PlayerSync(player.clone())),
                    ..Default::default()
                })
                .unwrap();
        });
}

fn broadcast_i_am_out_of_sync(
    server: ResMut<NetServer>,
    mut out_of_sync_events: EventReader<IAmOutOfSyncEvent>,
) {
    if out_of_sync_events.is_empty() {
        return;
    }

    for _ in out_of_sync_events.read() {}

    server
        .tx
        .send(applesauce::Wrapper {
            id: uuid::Uuid::new_v4().to_string(),
            inner: Some(Inner::OutOfSync(applesauce::OutOfSync::new())),
            ..Default::default()
        })
        .unwrap();
}

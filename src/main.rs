mod protos;
mod server;

use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::window::WindowPlugin;
use bevy::window::{PrimaryWindow, WindowResolution};
use crossbeam_channel::{Receiver, Sender};
use protos::generated::applesauce::wrapper::Inner;
use rand::prelude::*;

use protos::generated::applesauce;
use uuid::Uuid;

//
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
        .add_event::<InputEvent>()
        .add_event::<PlayerSpawnEvent>()
        .add_systems(Startup, setup)
        .add_systems(Startup, start_local_server)
        .add_systems(Update, ensure_main_player)
        .add_systems(Update, move_player)
        .add_systems(Update, fire_bullets)
        .add_systems(Update, bullet_moves_forward_system)
        .add_systems(Update, spawn_player)
        .add_systems(Update, bullet_hit_despawns_player)
        .add_systems(Update, write_inputs_to_server)
        .add_systems(Update, incoming_network_messages_to_events)
        .add_systems(Update, broadcast_state)
        .add_systems(Update, activate_shield)
        .add_systems(Update, shield_blocks_bullets)
        .add_systems(Update, despawn_shield_on_ttl)
        .run();
}

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Player {
    id: String,
    color: Vec3,
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
struct Bullet;

#[derive(Component)]
struct Shield {
    ttl: Timer,
}

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
    poll_timer: Timer,
}

#[derive(Event)]
struct InputEvent {
    player_id: String,

    move_left: bool,
    move_right: bool,

    aim_x: f32,
    aim_y: f32,
    fire_button_pressed: bool,
    shield_button_pressed: bool,
}

#[derive(Event)]
struct PlayerSpawnEvent {
    player_id: String,
    position: Vec3,
    color: Vec3,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
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
    let poll_timer = Timer::new(Duration::from_secs(1), TimerMode::Repeating);

    commands.insert_resource(NetServer { tx, rx, poll_timer });
}

fn incoming_network_messages_to_events(
    connection: ResMut<NetServer>,
    mut input_events: EventWriter<InputEvent>,
    mut player_spawn_events: EventWriter<PlayerSpawnEvent>,
) {
    for input in connection.rx.try_iter() {
        match input.inner.unwrap() {
            Inner::Input(input) => {
                input_events.send(InputEvent {
                    player_id: input.player_id,

                    move_left: input.move_left,
                    move_right: input.move_right,

                    aim_x: input.aim_x,
                    aim_y: input.aim_y,
                    fire_button_pressed: input.fire_button_pressed,
                    shield_button_pressed: input.shield_button_pressed,
                });
            }
            Inner::PlayerSpawn(player_spawn) => {
                player_spawn_events.send(PlayerSpawnEvent {
                    player_id: player_spawn.id,
                    position: Vec3::new(player_spawn.x, player_spawn.y, player_spawn.z),
                    color: Vec3::new(player_spawn.r, player_spawn.g, player_spawn.b),
                });
            }
            Inner::State(state) => {
                for player_spawn in state.players.iter() {
                    player_spawn_events.send(PlayerSpawnEvent {
                        player_id: player_spawn.id.clone(),
                        position: Vec3::new(player_spawn.x, player_spawn.y, player_spawn.z),
                        color: Vec3::new(player_spawn.r, player_spawn.g, player_spawn.b),
                    });
                }
            }
        }
    }
}

fn bullet_moves_forward_system(mut bullets: Query<&mut Transform, With<Bullet>>) {
    for mut bullet in bullets.iter_mut() {
        // move bullet forward, taking it's rotation into account
        let rotation = bullet.rotation * Vec3::X * 5.;
        bullet.translation += rotation;
    }
}

// keyboard
/// This system prints 'A' key state
fn move_player(mut players: Query<(&Player, &mut Transform)>, mut events: EventReader<InputEvent>) {
    for event in events.read() {
        let player = players
            .iter_mut()
            .find(|(player, _)| player.id == event.player_id);

        match player {
            None => continue,
            Some((_, mut transform)) => {
                if event.move_left {
                    transform.translation.x -= 1.;
                }

                if event.move_right {
                    transform.translation.x += 1.;
                }
            }
        }
    }
}

fn fire_bullets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut events: EventReader<InputEvent>,
    players: Query<(&Player, &Transform)>,
) {
    for event in events.read() {
        if !event.fire_button_pressed {
            continue;
        }

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
                println!(
                    "Just received a fire bullet event for a non extant player: {}",
                    event.player_id
                )
            }
        };
    }
}

fn activate_shield(
    mut commands: Commands,
    mut events: EventReader<InputEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    players: Query<(Entity, &Player)>,
) {
    for event in events.read() {
        if !event.shield_button_pressed {
            continue;
        }

        match players.iter().find(|(_, p)| p.id == event.player_id) {
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
            None => {
                println!("Shield activated for nonextant player: {}", event.player_id);
            }
        }
    }
}

fn ensure_main_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    main_players: Query<Entity, With<MainPlayer>>,
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
                    color: Vec3::new(r, g, b),
                },
                mesh_bundle: MaterialMesh2dBundle {
                    mesh: meshes.add(shape::Circle::new(50.).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::rgb(r, g, b))),
                    transform: Transform::from_translation(Vec3::new(x, 50., z)),
                    ..default()
                },
            },
        });
    }
}

fn spawn_player(
    mut commands: Commands,
    mut events: EventReader<PlayerSpawnEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut existing_players: Query<(&Player, &mut Transform)>,
) {
    for event in events.read() {
        match existing_players
            .iter_mut()
            .find(|(p, _)| p.id == event.player_id)
        {
            Some((_, mut transform)) => {
                transform.translation = event.position;
            }
            None => {
                commands.spawn(PlayerBundle {
                    player: Player {
                        id: event.player_id.clone(),
                        color: event.color,
                    },
                    mesh_bundle: MaterialMesh2dBundle {
                        mesh: meshes.add(shape::Circle::new(50.).into()).into(),
                        material: materials.add(ColorMaterial::from(Color::rgb(
                            event.color.x,
                            event.color.y,
                            event.color.z,
                        ))),
                        transform: Transform::from_translation(event.position),
                        ..default()
                    },
                });
            }
        }
    }
}

fn bullet_hit_despawns_player(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    mut players: Query<(Entity, &Transform), (With<Player>, Without<Shield>)>,
) {
    for (bullet, bloc) in bullets.iter() {
        for (entity, player) in players.iter_mut() {
            if bloc.translation.distance(player.translation) < 50. {
                commands.entity(entity).despawn();
                commands.entity(bullet).despawn();
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
                commands.entity(bullet).despawn();
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
            commands.entity(entity).despawn();
        }
    }
}

fn write_inputs_to_server(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    main_players: Query<(&Transform, &Player), With<MainPlayer>>,
    keyboard_input: Res<Input<KeyCode>>,
    server: Res<NetServer>,
) {
    write_inputs_to_server_fallible(
        windows,
        cameras,
        mouse_button_input,
        main_players,
        keyboard_input,
        server,
    );
}

fn write_inputs_to_server_fallible(
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    main_players: Query<(&Transform, &Player), With<MainPlayer>>,
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

    let move_left = keyboard_input.pressed(KeyCode::A);
    let move_right = keyboard_input.pressed(KeyCode::D);

    let (player_transform, player) = main_players.get_single().ok()?;
    let aim_vector = cursor_position - player_transform.translation;

    let fire_button_pressed = mouse_button_input.just_pressed(MouseButton::Left);
    let shield_button_pressed = mouse_button_input.just_pressed(MouseButton::Right);

    let wrapper = applesauce::Wrapper {
        id: Uuid::new_v4().into(),
        inner: Some(Inner::Input(applesauce::Input {
            player_id: player.id.clone(),
            move_left,
            move_right,
            aim_x: aim_vector.x,
            aim_y: aim_vector.y,
            fire_button_pressed,
            shield_button_pressed,
            ..Default::default()
        })),
        ..Default::default()
    };

    server.tx.send(wrapper).unwrap();
    Some(())
}

fn broadcast_state(
    mut server: ResMut<NetServer>,
    time: Res<Time>,
    players: Query<(&Player, &Transform)>,
) {
    server.poll_timer.tick(time.delta());

    if !server.poll_timer.finished() {
        return;
    }

    let players = players
        .iter()
        .map(|(player, transform)| applesauce::Player {
            id: player.id.clone(),
            x: transform.translation.x,
            y: transform.translation.y,
            z: transform.translation.z,
            r: player.color.x,
            g: player.color.y,
            b: player.color.z,

            ..Default::default()
        })
        .collect::<Vec<applesauce::Player>>();

    let state = applesauce::State {
        players,
        ..Default::default()
    };

    server
        .tx
        .send(applesauce::Wrapper {
            id: uuid::Uuid::new_v4().to_string(),
            inner: Some(Inner::State(state)),
            ..Default::default()
        })
        .unwrap();
}

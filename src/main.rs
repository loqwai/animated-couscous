#[macro_use]
extern crate derive_error;

mod level;
mod protos;
mod server;

use std::f32::consts::PI;
use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::utils::{HashMap, HashSet};
use bevy::window::WindowPlugin;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::plugin::{NoUserData, RapierPhysicsPlugin};
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::render::RapierDebugRenderPlugin;
use crossbeam_channel::{Receiver, Sender};
use level::PlayerSpawn;
use protos::generated::applesauce::wrapper::Inner;

use protos::generated::applesauce::{self};

const BULLET_SPEED: f32 = 1000.;
const PLAYER_MOVE_SPEED: f32 = 400.;
const FIRE_TIMEOUT: u64 = 500;
const JUMP_AMOUNT: f32 = 500.;
const GRAVITY: f32 = 2000.;
// How much to displace the bullet from the player so
// they don't shoot themselves if they're running towards
// where they're shooting
const FUDGE_FACTOR: f32 = 1.;

const WINDOW_WIDTH: f32 = 1000.;
const WINDOW_HEIGHT: f32 = 400.;

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
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugins(RapierDebugRenderPlugin::default())
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
                assign_main_player.after(handle_player_sync_events),
            ),
        )
        .add_systems(
            Update,
            (
                // optional debug systems
                // auto_fire,
                // debug_events,
                // Calculate next game state
                adjust_players_velocity,
                arc_bullets,
                bullet_hit_despawns_player_and_bullet,
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

#[derive(Component, Reflect)]
struct Player {
    id: String,
    spawn_id: u32,
    color: Color,
    radius: f32,
    fire_timeout: Timer,
}

#[derive(Component, Clone)]
struct MainPlayer(String);

#[derive(Bundle)]
struct PlayerBundle {
    name: Name,
    player: Player,
    velocity: Velocity,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
    body: RigidBody,
    collider: Collider,
    locked_axis: LockedAxes,
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

    // physics
    collider: Collider,
}

#[derive(Bundle)]
struct BulletBundle {
    name: Name,
    bullet: Bullet,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,

    // physics
    active_events: ActiveEvents,
    body: RigidBody,
    ccd: Ccd, // continuous collision detection
    collider: Collider,
    velocity: Velocity,
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

impl From<&applesauce::Bullet> for BulletSyncEvent {
    fn from(value: &applesauce::Bullet) -> Self {
        BulletSyncEvent {
            id: value.id.clone(),
            position: value.position.clone().unwrap().into(),
            velocity: value.velocity.clone().unwrap().into(),
        }
    }
}
impl From<applesauce::Bullet> for BulletSyncEvent {
    fn from(value: applesauce::Bullet) -> Self {
        (&value).into()
    }
}

#[derive(Event)]
struct PlayerSyncEvent {
    player_id: String,
    spawn_id: u32,
    position: Vec3,
    radius: f32,
    color: Color,
    move_data: MoveData,
}

impl From<&applesauce::Player> for PlayerSyncEvent {
    fn from(value: &applesauce::Player) -> PlayerSyncEvent {
        PlayerSyncEvent {
            player_id: value.id.clone(),
            spawn_id: value.spawn_id,
            position: value.position.clone().unwrap().into(),
            radius: value.radius,
            color: value.color.clone().unwrap().into(),
            move_data: MoveData {
                moving_left: value.move_data.moving_left,
                moving_right: value.move_data.moving_right,
            },
        }
    }
}
impl From<applesauce::Player> for PlayerSyncEvent {
    fn from(value: applesauce::Player) -> Self {
        (&value).into()
    }
}

#[derive(Event)]
struct BroadcastStateEvent;

#[derive(Event)]
struct IAmOutOfSyncEvent;

fn setup(mut commands: Commands) {
    commands.insert_resource(DeadList(HashSet::new()));
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(RapierConfiguration {
        gravity: Vec2::new(0., -GRAVITY).into(),
        ..Default::default()
    });
}

fn start_local_server(mut commands: Commands) {
    let listen_addr = std::env::var("SERVE_ON").unwrap_or("localhost:3191".to_string());
    let connect_addr = std::env::var("CONNECT_TO").unwrap_or("localhost:3191".to_string());

    let (tx, rx) = server::serve(&listen_addr, &connect_addr);

    commands.insert_resource(NetServer { tx, rx });
}

fn load_level(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    level::load_level(commands, meshes, materials, WINDOW_WIDTH, WINDOW_HEIGHT).unwrap();
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
    for e in player_sync_events.read() {
        println!("player_sync_event: {}", e.player_id);
    }
    for _ in block_events.read() {
        println!("block_event");
    }
    for e in despawn_player_events.read() {
        println!("despawn_player_event: {}", e.player_id);
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
            Inner::Player(player) => {
                player_spawn_events.send(player.into());
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
            Inner::Bullet(bullet) => {
                bullet_sync_events.send(bullet.into());
            }
            Inner::Jump(e) => jump_events.send(JumpEvent {
                player_id: e.player_id,
            }),
            Inner::State(state) => {
                state.players.iter().for_each(|player| {
                    player_spawn_events.send(player.into());
                });

                state.bullets.iter().for_each(|bullet| {
                    bullet_sync_events.send(bullet.into());
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
                        collider: Collider::ball(60.),
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
    bullets: Query<(&Bullet, &Transform, &Velocity)>,
    mut broadcast_state_events: EventReader<BroadcastStateEvent>,
) {
    if broadcast_state_events.is_empty() {
        return;
    }

    broadcast_state_events.clear();

    let state = applesauce::State {
        players: players
            .iter()
            .map(
                |(player, transform, move_left, move_right)| applesauce::Player {
                    id: player.id.clone(),
                    spawn_id: player.spawn_id,
                    position: applesauce::Vec3::from(transform.translation).into(),
                    color: applesauce::Color::from(player.color).into(),
                    radius: player.radius,
                    move_data: applesauce::MoveData::from((
                        move_left.is_some(),
                        move_right.is_some(),
                    ))
                    .into(),
                    special_fields: Default::default(),
                },
            )
            .collect(),
        bullets: bullets
            .iter()
            .map(|(bullet, transform, velocity)| applesauce::Bullet {
                id: bullet.id.clone(),
                position: applesauce::Vec3::from(transform.translation).into(),
                velocity: applesauce::Vec3::from(velocity.linvel).into(),
                special_fields: Default::default(),
            })
            .collect(),
        special_fields: Default::default(),
    };
    server.tx.send(state.into()).unwrap();
}

fn handle_bullet_sync_events(
    mut commands: Commands,
    mut events: EventReader<BulletSyncEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut bullets: Query<(&Bullet, &mut Transform, &mut Velocity)>,
    dead_list: Res<DeadList>,
) {
    for event in events.read() {
        if dead_list.0.contains(&event.id) {
            continue;
        }

        let rotation = Quat::from_rotation_z(event.velocity.y.atan2(event.velocity.x));

        match bullets.iter_mut().find(|(b, _, _)| b.id == event.id) {
            Some((_, mut transform, mut velocity)) => {
                transform.translation = event.position;
                transform.rotation = rotation;
                velocity.linvel = event.velocity.xy();
            }
            None => {
                commands.spawn(BulletBundle {
                    name: Name::new(format!("Bullet {}", event.id)),
                    bullet: Bullet {
                        id: uuid::Uuid::new_v4().to_string(),
                    },
                    active_events: ActiveEvents::COLLISION_EVENTS,
                    body: RigidBody::Dynamic,
                    ccd: Ccd::enabled(),
                    collider: Collider::cuboid(20., 5.),
                    velocity: Velocity::linear(event.velocity.xy()),
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
                velocity.linvel.y = JUMP_AMOUNT;
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
    dead_list: ResMut<DeadList>,
    server: ResMut<NetServer>,
) {
    let mut occupied_spawns: HashMap<u32, String> = existing_players
        .iter()
        .map(|(_, p, _)| (p.spawn_id, p.id.clone()))
        .collect();

    // it's possible to get two events in one frame to spawn the same player. The existing_players query
    // will then be out of date and we'll try to spawn the player twice. So we need to keep track of
    // which players we've already spawned this frame.
    let existing_player_ids: HashSet<String> = occupied_spawns.values().cloned().collect();

    for event in events.read() {
        if dead_list.0.contains(&event.player_id) {
            continue;
        }

        if let Some(occupying_player_id) = occupied_spawns.get(&event.spawn_id) {
            // we now have a collision. If the occupying player's ID is greather than the new, then
            // it dies and respawns. Otherwise, we just ignore the event.
            match occupying_player_id.cmp(&event.player_id) {
                std::cmp::Ordering::Less => {
                    server
                        .tx
                        .send(applesauce::DespawnPlayer::from(event.player_id.clone()).into())
                        .unwrap();
                    continue; // we just announced that this player should despawn, so no need to
                              // create it if it doesn't already exist
                }
                std::cmp::Ordering::Greater => server
                    .tx
                    .send(applesauce::DespawnPlayer::from(occupying_player_id.clone()).into())
                    .unwrap(),
                std::cmp::Ordering::Equal => {} // not really a collision, it's just us.
            }
        }

        let entity = match (
            existing_player_ids.contains(&event.player_id),
            existing_players
                .iter_mut()
                .find(|(_, p, _)| p.id == event.player_id),
        ) {
            (_, Some((entity, _, mut transform))) => {
                transform.translation = event.position;
                entity
            }
            (true, None) => {
                continue;
            } // we must have spawned this player already this frame
            (false, None) => {
                occupied_spawns.insert(event.spawn_id, event.player_id.clone());

                commands
                    .spawn(PlayerBundle {
                        name: Name::new(format!("Player {}", event.player_id)),
                        player: Player {
                            id: event.player_id.clone(),
                            spawn_id: event.spawn_id,
                            color: event.color,
                            radius: event.radius,
                            fire_timeout: Timer::new(
                                Duration::from_millis(FIRE_TIMEOUT),
                                TimerMode::Once,
                            ),
                        },
                        velocity: Velocity::zero(),
                        body: RigidBody::Dynamic,
                        collider: Collider::ball(event.radius),
                        locked_axis: LockedAxes::ROTATION_LOCKED,
                        mesh_bundle: MaterialMesh2dBundle {
                            mesh: meshes.add(shape::Circle::new(event.radius).into()).into(),
                            material: materials.add(ColorMaterial::from(event.color)),
                            transform: Transform::from_translation(event.position),
                            ..default()
                        },
                    })
                    .id()
            }
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

fn assign_main_player(
    mut commands: Commands,
    unassigned_main_players: Query<(Entity, &MainPlayer), Without<Player>>,
    players: Query<(Entity, &Player), Without<MainPlayer>>,
    dead_list: Res<DeadList>,
) {
    for (unassigned_main_player_entity, unassigned_main_player) in unassigned_main_players.iter() {
        if dead_list.0.contains(&unassigned_main_player.0) {
            println!(
                "Tried to assign main player to a dead player: {}",
                unassigned_main_player.0
            );

            commands
                .entity(unassigned_main_player_entity)
                .insert(Despawn);
            continue;
        }

        let player = players
            .iter()
            .find(|(_, p)| p.id == unassigned_main_player.0);

        if player.is_none() {
            println!(
                "Tried to assign main player to a player that doesn't exist yet: {}",
                unassigned_main_player.0
            );
            continue;
        }

        commands
            .entity(player.unwrap().0)
            .insert(unassigned_main_player.clone());

        commands
            .entity(unassigned_main_player_entity)
            .insert(Despawn);
    }
}

fn bullet_hit_despawns_player_and_bullet(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    bullets: Query<(Entity, &Bullet)>,
    players: Query<(Entity, &Player), Without<Shield>>,
    server: ResMut<NetServer>,
    mut dead_list: ResMut<DeadList>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Stopped(_, _, _) => continue,
            CollisionEvent::Started(e1, e2, _) => {
                let bullet = match bullets.iter().find(|(e, _)| e == e1 || e == e2) {
                    None => continue,
                    Some(b) => b,
                };

                commands.entity(bullet.0).insert(Despawn);
                dead_list.0.insert(bullet.1.id.clone());

                let player = match players.iter().find(|(e, _)| e == e1 || e == e2) {
                    None => continue,
                    Some(p) => p,
                };

                server
                    .tx
                    .send(applesauce::DespawnPlayer::from(player.1.id.clone()).into())
                    .unwrap();
            }
        }
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
    main_players: Query<Entity, (With<Player>, With<MainPlayer>)>,
    other_players: Query<&Player, Without<MainPlayer>>,
    player_spawns: Query<&PlayerSpawn>,
    server: Res<NetServer>,
) {
    if !main_players.is_empty() {
        return;
    }

    let id = uuid::Uuid::new_v4().to_string();

    let claimed_spawn_ids: HashSet<u32> = other_players.iter().map(|p| p.spawn_id).collect();

    // find a spawn that isn't already claimed
    let spawn = player_spawns
        .iter()
        .find(|s| claimed_spawn_ids.get(&s.id).is_none())
        .expect("Could not find an unclaimed spawn point");

    commands.spawn(MainPlayer(id.clone()));

    server
        .tx
        .send(
            applesauce::Player {
                id: id.clone(),
                spawn_id: spawn.id,
                position: applesauce::Vec3::from(spawn.position).into(),
                radius: spawn.radius,
                color: applesauce::Color::from(spawn.color).into(),
                move_data: applesauce::MoveData::from((false, false)).into(),
                special_fields: Default::default(),
            }
            .into(),
        )
        .unwrap();
}

fn adjust_players_velocity(
    mut left_movers: Query<&mut Velocity, (With<Player>, With<MoveLeft>, Without<MoveRight>)>,
    mut right_movers: Query<&mut Velocity, (With<Player>, With<MoveRight>, Without<MoveLeft>)>,
    mut non_movers: Query<&mut Velocity, (With<Player>, Without<MoveLeft>, Without<MoveRight>)>,
) {
    for mut left_mover in left_movers.iter_mut() {
        left_mover.linvel.x = -PLAYER_MOVE_SPEED;
    }

    for mut right_mover in right_movers.iter_mut() {
        right_mover.linvel.x = PLAYER_MOVE_SPEED;
    }

    for mut non_mover in non_movers.iter_mut() {
        non_mover.linvel.x = 0.;
    }
}

fn arc_bullets(mut bullets: Query<(&Transform, &mut Velocity), With<Bullet>>) {
    for (transform, mut velocity) in bullets.iter_mut() {
        let direction = velocity.linvel.normalize();
        let current_rotation = transform.rotation;

        // calculate the angle between the current direction and the direction of travel
        let (_, _, pitch) = current_rotation.to_euler(EulerRot::XYZ);
        let mut angle = direction.y.atan2(direction.x) - pitch;

        // if the angle is greater than PI, then we need to rotate the other way
        if angle.abs() > PI {
            angle = -angle;
        }

        // set the bullet's angular velocity so that it turns
        // towards the direction travel
        // the default angular velocity is too slow, we need it
        // to be faster without giving the bullet so much rotational
        // momentum that it starts flinging things across the screen
        // Therefore, multiply by 10 or so.
        velocity.angvel = angle * 10.;
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
                    spawn_id: player.spawn_id,
                    position: applesauce::Vec3::from(player_transform.translation).into(),
                    radius: player.radius,
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
    main_players: Query<(&mut Player, &Transform, &Velocity), With<MainPlayer>>,
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
    mut main_players: Query<(&mut Player, &Transform, &Velocity), With<MainPlayer>>,
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
    let offset = player.0.radius + bullet_half_length + FUDGE_FACTOR;
    let bullet_position = transform.translation.xy() + aim.clamp_length_min(offset);

    transform.translation = Vec3::new(bullet_position.x, bullet_position.y, 0.1);

    server
        .tx
        .send(
            applesauce::Bullet {
                id: uuid::Uuid::new_v4().to_string(),
                position: applesauce::Vec3::from(transform.translation).into(),
                velocity: applesauce::Vec3::from(
                    player.2.linvel + (aim.normalize() * BULLET_SPEED),
                )
                .into(),
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

use std::f32::consts::PI;

use bevy::{
    prelude::*,
    utils::{hashbrown::HashMap, HashSet},
};
use bevy_rapier2d::prelude::*;
use uuid::Uuid;

use crate::{
    events::{
        PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent, PlayerShootEvent,
        PlayerSpawnEvent,
    },
    level::{self, PlayerSpawn},
    AppConfig,
};

pub(crate) struct ManageStatePlugin;

impl Plugin for ManageStatePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
            .add_plugins(RapierDebugRenderPlugin::default())
            .add_event::<GameStateEvent>()
            .add_event::<PlayerSpawnEvent>()
            .add_event::<PlayerMoveLeftEvent>()
            .add_event::<PlayerMoveRightEvent>()
            .add_event::<PlayerJumpEvent>()
            .add_event::<PlayerShootEvent>()
            .add_systems(Startup, (load_level, configure_gravity))
            .add_systems(
                First,
                (
                    reset_player_horizontal_velocity,
                    reset_vertical_impulse,
                    update_players_from_game_state_event,
                    update_bullets_from_game_state_event,
                ),
            )
            .add_systems(
                PreUpdate,
                (
                    handle_player_spawn_event,
                    handle_player_move_left_event,
                    handle_player_move_right_event,
                    handle_player_jump_event,
                    handle_player_shoot_event,
                ),
            )
            .add_systems(
                Update,
                (
                    arc_bullets,
                    bullets_despawn_on_collision_with_anything,
                    players_despawn_on_collision_with_bullets,
                ),
            )
            .add_systems(PostUpdate, despawn_things_that_need_despawning);
    }
}

#[derive(Event)]
pub(crate) struct GameStateEvent {
    pub(crate) timestamp: u64,
    pub(crate) players: Vec<PlayerState>,
    pub(crate) bullets: Vec<BulletState>,
}

pub(crate) struct PlayerState {
    pub(crate) id: String,
    pub(crate) client_id: String,
    pub(crate) spawn_id: String,
    pub(crate) radius: f32,
    pub(crate) color: Color,
    pub(crate) position: Vec3,
}

pub(crate) struct BulletState {
    pub(crate) id: String,
    pub(crate) transform: Transform,
    pub(crate) velocity: Vec2,
}

#[derive(Component, Reflect)]
struct Despawn;

#[derive(Component, Default, Reflect)]
pub(crate) struct Player {
    pub(crate) id: String,
    pub(crate) spawn_id: String,
    pub(crate) client_id: String,
    pub(crate) radius: f32,
    pub(crate) color: Color,
}

#[derive(Bundle)]
struct PlayerBundle {
    name: Name,
    player: Player,
    rigid_body: RigidBody,
    collider: Collider,
    transform: TransformBundle,
    velocity: Velocity,
    external_impulse: ExternalImpulse,
    locked_axes: LockedAxes,
}

impl PlayerBundle {
    pub(crate) fn new(player: Player, transform: Transform) -> Self {
        Self {
            name: Name::new(format!("Player {}", player.client_id)),
            collider: Collider::ball(player.radius),
            player,
            rigid_body: RigidBody::Dynamic,
            transform: TransformBundle::from_transform(transform),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            velocity: Velocity::default(),
            external_impulse: Default::default(),
        }
    }
}

#[derive(Component, Reflect)]
pub(crate) struct Bullet {
    pub(crate) id: String,
}

#[derive(Bundle)]
struct BulletBundle {
    bullet: Bullet,
    rigid_body: RigidBody,
    collider: Collider,
    transform: TransformBundle,
    velocity: Velocity,
    active_events: ActiveEvents,
}

impl BulletBundle {
    pub(crate) fn new(bullet: Bullet, transform: Transform, velocity: Vec2) -> Self {
        Self {
            bullet,
            transform: TransformBundle::from_transform(transform),
            velocity: Velocity::linear(velocity),
            rigid_body: RigidBody::Dynamic,
            collider: Collider::cuboid(20., 5.),
            active_events: ActiveEvents::COLLISION_EVENTS,
        }
    }
}

fn load_level(
    commands: Commands,
    config: Res<AppConfig>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    level::load_level(commands, meshes, materials, config.width, config.height)
        .expect("Failed to load level");
}

fn configure_gravity(mut commands: Commands, config: Res<AppConfig>) {
    commands.insert_resource(RapierConfiguration {
        gravity: Vec2::new(0., -config.gravity).into(),
        ..default()
    });
}

fn reset_player_horizontal_velocity(mut velocities: Query<&mut Velocity, With<Player>>) {
    for mut velocity in velocities.iter_mut() {
        velocity.linvel.x = 0.;
    }
}

fn reset_vertical_impulse(mut impulses: Query<&mut ExternalImpulse>) {
    for mut impulse in impulses.iter_mut() {
        impulse.impulse.y = 0.;
    }
}

fn update_players_from_game_state_event(
    mut commands: Commands,
    mut players: Query<(Entity, &Player, &mut Transform)>,
    mut events: EventReader<GameStateEvent>,
) {
    match events.read().max_by(|a, b| a.timestamp.cmp(&b.timestamp)) {
        None => return,
        Some(game_state) => {
            let mut player_entities_by_id: HashMap<String, Entity> = players
                .iter_mut()
                .map(|(entity, player, _)| (player.id.to_string(), entity))
                .collect();

            for player_state in game_state.players.iter() {
                player_entities_by_id.remove(&player_state.id);

                match players.iter_mut().find(|(_, b, _)| b.id == player_state.id) {
                    Some((_, _, mut transform)) => {
                        transform.translation = player_state.position.clone();
                    }
                    None => {
                        commands.spawn(PlayerBundle::new(
                            Player {
                                id: player_state.id.clone(),
                                spawn_id: player_state.spawn_id.clone(),
                                client_id: player_state.client_id.clone(),
                                radius: player_state.radius,
                                color: player_state.color.clone(),
                            },
                            Transform::from_translation(player_state.position.clone()),
                        ));
                    }
                }
            }

            for (_, entity) in player_entities_by_id {
                commands.entity(entity).insert(Despawn);
            }
        }
    }
}

fn update_bullets_from_game_state_event(
    mut commands: Commands,
    mut bullets: Query<(Entity, &Bullet, &mut Transform)>,
    mut events: EventReader<GameStateEvent>,
) {
    match events.read().max_by(|a, b| a.timestamp.cmp(&b.timestamp)) {
        None => return,
        Some(game_state) => {
            let mut bullet_entities_by_id: HashMap<String, Entity> = bullets
                .iter_mut()
                .map(|(entity, bullet, _)| (bullet.id.to_string(), entity))
                .collect();

            for bullet_state in game_state.bullets.iter() {
                bullet_entities_by_id.remove(&bullet_state.id);

                match bullets.iter_mut().find(|(_, b, _)| b.id == bullet_state.id) {
                    Some((_, _, mut transform)) => {
                        transform.set_if_neq(bullet_state.transform);
                    }
                    None => {
                        commands.spawn(BulletBundle::new(
                            Bullet {
                                id: bullet_state.id.clone(),
                            },
                            bullet_state.transform.clone(),
                            bullet_state.velocity.clone(),
                        ));
                    }
                }
            }

            for (_, entity) in bullet_entities_by_id {
                commands.entity(entity).insert(Despawn);
            }
        }
    }
}

fn handle_player_spawn_event(
    mut commands: Commands,
    mut events: EventReader<PlayerSpawnEvent>,
    players: Query<&Player>,
    spawns: Query<&PlayerSpawn>,
) {
    let spawned_client_ids: HashSet<String> =
        players.iter().map(|p| p.client_id.to_string()).collect();
    let used_spawn_ids: HashSet<String> = players.iter().map(|p| p.spawn_id.to_string()).collect();
    let unused_spawns: Vec<&PlayerSpawn> = spawns
        .iter()
        .filter(|s| !used_spawn_ids.contains(&s.id.to_string()))
        .collect::<Vec<&PlayerSpawn>>();

    let mut unused_spawns_iter = unused_spawns.iter();

    for event in events.read() {
        if spawned_client_ids.contains(&event.client_id.to_string()) {
            println!(
                "Ignoring spawn player event from client that already has a player. client-id: {}.",
                event.client_id
            );
            continue;
        }

        match unused_spawns_iter.next() {
            None => return,
            Some(spawn) => {
                commands.spawn(PlayerBundle::new(
                    Player {
                        id: Uuid::new_v4().to_string(),
                        spawn_id: spawn.id.to_string(),
                        client_id: event.client_id.to_string(),
                        radius: spawn.radius,
                        color: spawn.color,
                    },
                    Transform::from_translation(spawn.position),
                ));
            }
        }
    }
}

fn handle_player_move_left_event(
    config: Res<AppConfig>,
    mut players: Query<(&Player, &mut Velocity)>,
    mut events: EventReader<PlayerMoveLeftEvent>,
) {
    for event in events.read() {
        match players
            .iter_mut()
            .find(|(p, _)| p.client_id == event.client_id)
        {
            None => continue,
            Some((_, mut velocity)) => velocity.linvel.x += -config.player_move_speed,
        }
    }
}

fn handle_player_move_right_event(
    config: Res<AppConfig>,
    mut players: Query<(&Player, &mut Velocity)>,
    mut events: EventReader<PlayerMoveRightEvent>,
) {
    for event in events.read() {
        match players
            .iter_mut()
            .find(|(p, _)| p.client_id == event.client_id)
        {
            None => continue,
            Some((_, mut velocity)) => velocity.linvel.x += config.player_move_speed,
        }
    }
}

fn handle_player_jump_event(
    config: Res<AppConfig>,
    mut players: Query<(&Player, &mut ExternalImpulse)>,
    mut events: EventReader<PlayerJumpEvent>,
) {
    // TODO: Ignore jump events from players that aren't touching the ground

    for event in events.read() {
        match players
            .iter_mut()
            .find(|(p, _)| p.client_id == event.client_id)
        {
            None => continue,
            Some((_, mut impulse)) => impulse.impulse.y += config.jump_amount,
        }
    }
}

fn handle_player_shoot_event(
    mut commands: Commands,
    mut events: EventReader<PlayerShootEvent>,
    players: Query<(&Player, &Transform)>,
    config: Res<AppConfig>,
) {
    for event in events.read() {
        match players.iter().find(|(p, _)| p.client_id == event.client_id) {
            None => continue,
            Some((player, transform)) => {
                // TODO: Implement fire timeout

                let bullet_half_length = 20.;
                let offset = player.radius + bullet_half_length + config.fudge_factor;
                let bullet_position =
                    transform.translation.xy() + event.aim.clamp_length_min(offset);

                let velocity = Vec2::from(event.aim.normalize() * config.bullet_speed);
                let rotation = Quat::from_rotation_z(velocity.y.atan2(velocity.x));

                commands.spawn(BulletBundle::new(
                    Bullet {
                        id: Uuid::new_v4().to_string(),
                    },
                    Transform {
                        translation: Vec3::new(bullet_position.x, bullet_position.y, 0.1),
                        rotation,
                        ..default()
                    },
                    velocity,
                ));
            }
        };
    }
}

fn arc_bullets(mut bullets: Query<(&Transform, &mut Velocity), With<Bullet>>) {
    for (transform, mut velocity) in bullets.iter_mut() {
        let direction = velocity.linvel.normalize();
        let current_rotation = transform.rotation;

        // calculate the angle between the current direction and the direction of travel
        let (_, _, pitch) = current_rotation.to_euler(EulerRot::XYZ);
        let mut angle = direction.y.atan2(direction.x) - pitch;

        // angle is now a value between -2 * PI and 2 * PI. We want to normalize it
        // to be between -PI and PI
        if angle > PI {
            angle -= 2. * PI;
        } else if angle < -PI {
            angle += 2. * PI;
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

fn bullets_despawn_on_collision_with_anything(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    bullets: Query<Entity, With<Bullet>>,
) {
    for collision in collision_events.read() {
        match collision {
            CollisionEvent::Stopped(_, _, _) => continue,
            CollisionEvent::Started(e1, e2, _) => {
                if bullets.get(*e1).is_ok() {
                    commands.entity(*e1).insert(Despawn);
                }

                if bullets.get(*e2).is_ok() {
                    commands.entity(*e2).insert(Despawn);
                }
            }
        }
    }
}

fn players_despawn_on_collision_with_bullets(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    players: Query<Entity, With<Player>>,
    bullets: Query<Entity, With<Bullet>>,
) {
    for collision in collision_events.read() {
        match collision {
            CollisionEvent::Stopped(_, _, _) => continue,
            CollisionEvent::Started(e1, e2, _) => {
                if bullets.get(*e1).or_else(|_| bullets.get(*e2)).is_err() {
                    continue;
                }

                let player = match players.get(*e1).or_else(|_| players.get(*e2)) {
                    Ok(player) => player,
                    Err(_) => continue,
                };

                commands.entity(player).insert(Despawn);
            }
        }
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

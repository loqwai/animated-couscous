use bevy::{prelude::*, utils::HashSet};
use bevy_rapier2d::prelude::*;

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
            .add_event::<PlayerSpawnEvent>()
            .add_event::<PlayerMoveLeftEvent>()
            .add_event::<PlayerMoveRightEvent>()
            .add_event::<PlayerJumpEvent>()
            .add_event::<PlayerShootEvent>()
            .add_systems(Startup, (load_level, configure_gravity))
            .add_systems(
                First,
                (reset_player_horizontal_velocity, reset_vertical_impulse),
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
            );
    }
}

#[derive(Component, Reflect)]
pub(crate) struct Player {
    // id: String,
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

#[derive(Component, Reflect)]
pub(crate) struct Bullet;

#[derive(Bundle)]
struct BulletBundle {
    bullet: Bullet,
    rigid_body: RigidBody,
    collider: Collider,
    transform: TransformBundle,
    velocity: Velocity,
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
            None => {
                println!("No more spawn points available");
                return;
            }
            Some(spawn) => {
                println!("Spawn player at spawn point {}", spawn.id);
                commands.spawn(PlayerBundle {
                    name: Name::new(format!("Player {}", event.client_id)),
                    player: Player {
                        // id: Uuid::new_v4().to_string(),
                        spawn_id: spawn.id.to_string(),
                        client_id: event.client_id.to_string(),
                        radius: spawn.radius,
                        color: Color::rgb(1., 0., 0.),
                    },
                    rigid_body: RigidBody::Dynamic,
                    collider: Collider::ball(spawn.radius),
                    transform: TransformBundle::from_transform(Transform::from_translation(
                        spawn.position,
                    )),
                    velocity: Velocity::default(),
                    external_impulse: ExternalImpulse::default(),
                    locked_axes: LockedAxes::ROTATION_LOCKED,
                });
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
                let bullet_half_length = 20.;
                let offset = player.radius + bullet_half_length + config.fudge_factor;
                let bullet_position =
                    transform.translation.xy() + event.aim.clamp_length_min(offset);

                let velocity = Vec2::from(event.aim.normalize() * config.bullet_speed);
                let rotation = Quat::from_rotation_z(velocity.y.atan2(velocity.x));

                commands.spawn(BulletBundle {
                    bullet: Bullet,
                    rigid_body: RigidBody::Dynamic,
                    collider: Collider::cuboid(20., 5.),
                    transform: TransformBundle::from_transform(Transform {
                        translation: Vec3::new(bullet_position.x, bullet_position.y, 0.1),
                        rotation,
                        ..default()
                    }),
                    velocity: Velocity::linear(velocity),
                });
            }
        };
    }
}

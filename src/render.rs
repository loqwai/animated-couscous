use bevy::{prelude::*, sprite::MaterialMesh2dBundle, utils::HashSet};

use crate::game_state::{BulletState, GameStateEvent, PlayerState};

pub(crate) struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GameStateEvent>();
        app.add_systems(Update, update_render_components_from_state);
    }
}

#[derive(Component)]
struct Player {
    id: String,
}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

#[derive(Component)]
struct Bullet {
    id: String,
}

#[derive(Bundle)]
struct BulletBundle {
    bullet: Bullet,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

fn update_render_components_from_state(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    states: EventReader<GameStateEvent>,
    players: Query<(Entity, &Player, &mut Transform), Without<Bullet>>,
    bullets: Query<(Entity, &Bullet, &mut Transform), Without<Player>>,
) {
    update_render_components_from_state_fallible(
        commands, meshes, materials, states, players, bullets,
    );
}

fn update_render_components_from_state_fallible(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut states: EventReader<GameStateEvent>,
    mut players: Query<(Entity, &Player, &mut Transform), Without<Bullet>>,
    mut bullets: Query<(Entity, &Bullet, &mut Transform), Without<Player>>,
) -> Option<()> {
    let state = states.read().max_by(|a, b| a.timestamp.cmp(&b.timestamp))?;

    remove_stale_players(&mut commands, &players, &state);
    for player_state in state.players.iter() {
        handle_player_state(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut players,
            player_state,
        )
    }

    remove_stale_bullets(&mut commands, &bullets, &state);
    for bullet_state in state.bullets.iter() {
        handle_bullet_state(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut bullets,
            bullet_state,
        );
    }

    None
}

fn remove_stale_players(
    commands: &mut Commands,
    players: &Query<(Entity, &Player, &mut Transform), Without<Bullet>>,
    state: &GameStateEvent,
) {
    let active_player_ids: HashSet<&String> = state.players.iter().map(|p| &p.id).collect();

    players
        .iter()
        .filter(|p| !active_player_ids.contains(&p.1.id))
        .for_each(|p| commands.entity(p.0).despawn());
}

fn handle_player_state(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    players: &mut Query<(Entity, &Player, &mut Transform), Without<Bullet>>,
    player_state: &PlayerState,
) {
    match players.iter_mut().find(|p| p.1.id == player_state.id) {
        Some(mut p) => p.2.clone_from(&&player_state.transform),
        None => {
            commands.spawn(PlayerBundle {
                player: Player {
                    id: player_state.id.clone(),
                },
                mesh_bundle: MaterialMesh2dBundle {
                    mesh: meshes
                        .add(shape::Circle::new(player_state.radius).into())
                        .into(),
                    material: materials.add(ColorMaterial::from(player_state.color)),
                    transform: player_state.transform,
                    ..default()
                },
            });
        }
    }
}

fn remove_stale_bullets(
    commands: &mut Commands,
    bullets: &Query<(Entity, &Bullet, &mut Transform), Without<Player>>,
    state: &GameStateEvent,
) {
    let active_player_ids: HashSet<&String> = state.bullets.iter().map(|p| &p.id).collect();

    bullets
        .iter()
        .filter(|p| !active_player_ids.contains(&p.1.id))
        .for_each(|p| commands.entity(p.0).despawn());
}

fn handle_bullet_state(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    bullets: &mut Query<(Entity, &Bullet, &mut Transform), Without<Player>>,
    bullet_state: &BulletState,
) {
    match bullets.iter_mut().find(|b| b.1.id == bullet_state.id) {
        Some(mut b) => b.2.clone_from(&&bullet_state.transform),
        None => {
            commands.spawn(BulletBundle {
                bullet: Bullet {
                    id: bullet_state.id.clone(),
                },
                mesh_bundle: MaterialMesh2dBundle {
                    mesh: meshes
                        .add(shape::Quad::new(Vec2::new(40., 10.)).into())
                        .into(),
                    material: materials.add(ColorMaterial::from(Color::WHITE)),
                    transform: bullet_state.transform,
                    ..default()
                },
            });
        }
    }
}

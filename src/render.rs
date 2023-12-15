use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};

use crate::manage_state::{Bullet, Player};

pub(crate) struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, (ensure_players_render, ensure_bullets_render));
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn ensure_players_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    players: Query<(Entity, &Player, &Transform), Without<Mesh2dHandle>>,
) {
    for (entity, player, transform) in players.iter() {
        commands.entity(entity).insert(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(player.radius).into()).into(),
            material: materials.add(ColorMaterial::from(player.color)),
            transform: transform.clone(),
            ..default()
        });
    }
}

fn ensure_bullets_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    bullets: Query<(Entity, &Transform), (With<Bullet>, Without<Mesh2dHandle>)>,
) {
    for (entity, transform) in bullets.iter() {
        commands.entity(entity).insert(MaterialMesh2dBundle {
            mesh: meshes
                .add(shape::Quad::new(Vec2::new(40., 10.)).into())
                .into(),
            material: materials.add(ColorMaterial::from(Color::WHITE)),
            transform: transform.clone(),
            ..default()
        });
    }
}

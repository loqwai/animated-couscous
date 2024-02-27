use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    text::Text2dBounds,
};

use crate::{
    manage_state::{Bullet, Gun, Health, Player, Shield},
    AppConfig,
};

pub(crate) struct RenderPlugin;

#[derive(Component)]
pub(crate) struct AmmoCountDisplay;

#[derive(Component)]
pub(crate) struct HealthDisplay;

#[derive(Component)]
pub(crate) struct HasHealthDisplay;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(
            Update,
            (
                ensure_players_render,
                ensure_things_with_health_have_health_display,
                ensure_bullets_render,
                ensure_shields_render,
                ensure_guns_render,
                render_ammo_count,
                render_health,
            ),
        );
    }
}

fn render_ammo_count(mut query: Query<(&Gun, &mut Text), With<AmmoCountDisplay>>) {
    for (gun, mut text) in query.iter_mut() {
        text.sections[0].value = format!("{}", gun.bullet_count);
    }
}

fn render_health(
    mut health_displays: Query<(&mut Text, &Parent), With<HealthDisplay>>,
    healths: Query<&Health>,
    config: Res<AppConfig>,
) {
    for (mut text, parent) in health_displays.iter_mut() {
        let health = match healths.get(**parent) {
            Err(_) => continue,
            Ok(health) => health,
        };

        text.sections[0].value = format!("{}/{}", health.0, config.player_health);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn ensure_guns_render(
    mut commands: Commands,
    guns: Query<(Entity, &Transform), (With<Gun>, Without<Text>)>,
) {
    for (entity, transform) in guns.iter() {
        commands.entity(entity).insert((
            AmmoCountDisplay,
            Text2dBundle {
                text: Text {
                    sections: vec![TextSection::new(
                        "HI",
                        TextStyle {
                            font_size: 20.,
                            color: Color::WHITE,
                            ..Default::default()
                        },
                    )],
                    ..default()
                },
                text_2d_bounds: Text2dBounds {
                    size: Vec2::new(100., 100.),
                    ..default()
                },
                transform: transform.clone(),
                ..default()
            },
        ));
    }
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

fn ensure_things_with_health_have_health_display(
    mut commands: Commands,
    players: Query<Entity, (With<Health>, Without<HasHealthDisplay>)>,
) {
    for entity in players.iter() {
        commands
            .entity(entity)
            .with_children(|parent| {
                parent.spawn((
                    HealthDisplay,
                    Text2dBundle {
                        text: Text {
                            sections: vec![TextSection::new(
                                "HI",
                                TextStyle {
                                    font_size: 20.,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            )],
                            ..default()
                        },
                        text_2d_bounds: Text2dBounds {
                            size: Vec2::new(100., 100.),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(0., 50., 0.)),
                        ..default()
                    },
                ));
            })
            .insert(HasHealthDisplay);
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

fn ensure_shields_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    shields: Query<(Entity, &Shield, &Transform), Without<Mesh2dHandle>>,
) {
    for (entity, shield, transform) in shields.iter() {
        commands.entity(entity).insert(MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(shield.radius).into()).into(),
            material: materials.add(ColorMaterial::from(Color::rgba(1., 1., 1., 0.1))),
            transform: transform.clone(),
            ..default()
        });
    }
}

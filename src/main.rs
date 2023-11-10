mod protos;

use std::net::{TcpListener, TcpStream};

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::window::PrimaryWindow;
use rand::prelude::*;

use protos::generated::applesauce;

//
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Startup, start_local_server)
        .add_systems(Startup, maybe_connect_to_remote_server)
        .add_systems(Update, keyboard_input_system)
        .add_systems(Update, mouse_click_system)
        .add_systems(Update, bullet_moves_forward_system)
        .add_systems(Update, ensure_dummy)
        .add_systems(Update, bullet_hit_despawns_dummy)
        .add_systems(Update, write_inputs_to_network)
        .run();
}

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Player;

// dummy component
#[derive(Component)]
struct Dummy;

#[derive(Bundle)]
struct DummyBundle {
    dummy: Dummy,
    name: Name,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

#[derive(Component)]
struct Bullet;

#[derive(Bundle)]
struct BulletBundle {
    bullet: Bullet,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
}

#[derive(Resource)]
struct NetworkConnection(TcpStream);

#[derive(Resource)]
struct NetServer(TcpListener);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    // Player
    commands.spawn(PlayerBundle {
        player: Player,
        mesh_bundle: MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(50.).into()).into(),
            material: materials.add(ColorMaterial::from(Color::BLUE)),
            transform: Transform::from_translation(Vec3::new(0., 50., 0.)),
            ..default()
        },
    });

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
    let listener = TcpListener::bind("0.0.0.0:3191").unwrap();
    commands.insert_resource(NetServer(listener));
}

fn maybe_connect_to_remote_server(mut commands: Commands) {
    let server = std::env::var("REMOTE_SERVER").unwrap_or("localhost:3191".to_string());
    let connection = TcpStream::connect(server).unwrap();
    commands.insert_resource(NetworkConnection(connection));
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
fn keyboard_input_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut players: Query<&mut Transform, With<Player>>,
) {
    let mut player = players.get_single_mut().unwrap();

    if keyboard_input.pressed(KeyCode::A) {
        player.translation.x -= 1.;
    }

    if keyboard_input.pressed(KeyCode::D) {
        player.translation.x += 1.;
    }
}

fn mouse_click_system(
    commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    players: Query<&Transform, With<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    mouse_click_system_fallible(
        commands,
        mouse_button_input,
        players,
        windows,
        cameras,
        meshes,
        materials,
    );
}

// This system prints messages when you press or release the left mouse button:
fn mouse_click_system_fallible(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    players: Query<&Transform, With<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) -> Option<()> {
    let (camera, camera_transform) = cameras.get_single().ok()?;
    let cursor = windows.get_single().ok()?.cursor_position()?;
    let cursor_position = camera.viewport_to_world(camera_transform, cursor)?.origin;

    let player = players.get_single().ok()?;

    if mouse_button_input.just_pressed(MouseButton::Left) {
        let ray = cursor_position - player.translation;
        let rotation = Quat::from_rotation_z(ray.y.atan2(ray.x));
        let mut transform = player.clone().with_rotation(rotation);
        transform.translation.z += 0.1;

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
    return Some(());
}

fn ensure_dummy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    dummies: Query<&Name, With<Dummy>>,
) {
    if dummies.iter().count() == 0 {
        let mut rng = rand::thread_rng();
        let mut random_position: f32 = rng.gen();
        random_position *= 1000.;
        random_position -= 500.;

        commands.spawn(DummyBundle {
            dummy: Dummy,
            name: Name("Dummy".to_string()),
            mesh_bundle: MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::new(50.).into()).into(),
                material: materials.add(ColorMaterial::from(Color::RED)),
                transform: Transform::from_translation(Vec3::new(random_position, 50., -0.1)),
                ..default()
            },
        });
    }
}

fn bullet_hit_despawns_dummy(
    mut commands: Commands,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    mut dummies: Query<(Entity, &Transform), With<Dummy>>,
) {
    for (bullet, bloc) in bullets.iter() {
        for (entity, dummy) in dummies.iter_mut() {
            if bloc.translation.distance(dummy.translation) < 70. {
                commands.entity(entity).despawn();
                commands.entity(bullet).despawn();
            }
        }
    }
}

fn write_inputs_to_network(mut connection: ResMut<NetworkConnection>) {}

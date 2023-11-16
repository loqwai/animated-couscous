mod protos;
mod server;

use std::net::{TcpListener, TcpStream};
use std::thread;

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::window::PrimaryWindow;
use crossbeam_channel::Receiver;
use protobuf::{CodedInputStream, Message};
use rand::prelude::*;

use protos::generated::applesauce;

//
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_event::<InputEvent>()
        .add_systems(Startup, setup)
        .add_systems(Startup, start_local_server)
        .add_systems(Startup, connect_to_remote_server)
        .add_systems(Update, move_player)
        .add_systems(Update, maybe_fire_bullet)
        .add_systems(Update, bullet_moves_forward_system)
        .add_systems(Update, ensure_dummy)
        .add_systems(Update, bullet_hit_despawns_dummy)
        .add_systems(Update, write_inputs_to_network)
        .add_systems(Update, broadcast_incoming_events)
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
struct NetworkConnection {
    stream: TcpStream,
    channel: Receiver<InputEvent>,
}

#[derive(Resource)]
struct NetServer(TcpListener);

#[derive(Event)]
struct InputEvent {
    move_left: bool,
    move_right: bool,

    aim_x: f32,
    aim_y: f32,
    fire_button_pressed: bool,
}

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

    commands.insert_resource(NetServer(listener.try_clone().unwrap()));

    thread::spawn(move || server::serve(listener));
}

fn connect_to_remote_server(mut commands: Commands) {
    let server = std::env::var("REMOTE_SERVER").unwrap_or("localhost:3191".to_string());
    let mut connection = TcpStream::connect(server).unwrap();

    let (tx, rx) = crossbeam_channel::bounded::<InputEvent>(10);

    commands.insert_resource(NetworkConnection {
        stream: connection.try_clone().unwrap(),
        channel: rx,
    });

    thread::spawn(move || {
        let mut input_stream = CodedInputStream::new(&mut connection);

        loop {
            let input: applesauce::Input = input_stream.read_message().unwrap();
            tx.send(InputEvent {
                move_left: input.move_left,
                move_right: input.move_right,

                aim_x: input.aim_x,
                aim_y: input.aim_y,
                fire_button_pressed: input.fire_button_pressed,
            })
            .unwrap();
        }
    });
}

fn broadcast_incoming_events(
    connection: ResMut<NetworkConnection>,
    mut events: EventWriter<InputEvent>,
) {
    for event in connection.channel.try_iter() {
        events.send(event);
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
fn move_player(
    mut players: Query<&mut Transform, With<Player>>,
    mut events: EventReader<InputEvent>,
) {
    let mut player = players.get_single_mut().unwrap();

    for event in events.read() {
        if event.move_left {
            player.translation.x -= 1.;
        }

        if event.move_right {
            player.translation.x += 1.;
        }
    }
}

fn maybe_fire_bullet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut events: EventReader<InputEvent>,
    players: Query<&Transform, With<Player>>,
) {
    let player = players.get_single().unwrap();

    for event in events.read() {
        if !event.fire_button_pressed {
            continue;
        }

        let ray = Vec3::new(event.aim_x, event.aim_y, 0.);
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

fn write_inputs_to_network(
    mut connection: ResMut<NetworkConnection>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mouse_button_input: Res<Input<MouseButton>>,
    players: Query<&Transform, With<Player>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if windows.get_single().unwrap().cursor_position().is_none() {
        return;
    }

    let (camera, camera_transform) = cameras.get_single().unwrap();
    let cursor = windows.get_single().unwrap().cursor_position().unwrap();
    let cursor_position = camera
        .viewport_to_world(camera_transform, cursor)
        .unwrap()
        .origin;

    let move_left = keyboard_input.pressed(KeyCode::A);
    let move_right = keyboard_input.pressed(KeyCode::D);

    let player = players.get_single().unwrap();
    let aim_vector = cursor_position - player.translation;

    let fire_button_pressed = mouse_button_input.just_pressed(MouseButton::Left);

    let input = applesauce::Input {
        move_left,
        move_right,
        aim_x: aim_vector.x,
        aim_y: aim_vector.y,
        fire_button_pressed,
        ..Default::default()
    };

    input
        .write_length_delimited_to_writer(&mut connection.stream)
        .unwrap();
}

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, keyboard_input_system)
        .run();
}

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Player;

#[derive(Bundle)]
struct PlayerBundle {
    player: Player,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
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
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            transform: Transform::from_translation(Vec3::new(0., 25., 0.)),
            ..default()
        },
    });

    // Ground
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::Quad::new(Vec2::new(1000., 1000.)).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::DARK_GREEN)),
        transform: Transform::from_translation(Vec3::new(0., -500., -0.1)),
        ..default()
    });
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

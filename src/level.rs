use std::collections::HashMap;

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;

const LEVEL_PATH: &str = "assets/level.svg";

const PLAYER_SPAWN_IDS: [&str; 2] = ["player1Spawn", "player2Spawn"];

#[derive(Component)]
pub(crate) struct PlayerSpawn {
    pub player_number: u8,
    pub position: Vec3,
    pub color: Color,
}

pub(crate) fn load_level(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut content = String::new();
    let parser = svg::open(LEVEL_PATH, &mut content).unwrap();
    for event in parser {
        match event {
            svg::parser::Event::Error(e) => panic!("Error parsing SVG: {:?}", e),
            svg::parser::Event::Tag(path, _tag_type, attributes) => {
                if let Some(id) = attributes.get("id") {
                    if PLAYER_SPAWN_IDS.contains(&id.to_string().as_str()) {
                        handle_player_spawn(&mut commands, id, path, &attributes);
                        continue;
                    }
                }

                println!("Found tag {}", path);
            }
            _ => {}
        }
    }

    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::Quad::new(Vec2::new(1000., 1000.)).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::GRAY)),
        transform: Transform::from_translation(Vec3::new(0., -500., -0.1)),
        ..default()
    });
}

fn handle_player_spawn(
    commands: &mut Commands,
    id: &str,
    path: &str,
    attributes: &HashMap<String, svg::node::Value>,
) {
    if path != "circle" {
        panic!("player_spawn {} is not a circle", id);
    }

    let player_number = match id {
        "player1Spawn" => 1,
        "player2Spawn" => 2,
        _ => panic!("Unknown player spawn id {}", id),
    };

    let x: f32 = attributes
        .get("cx")
        .expect(&format!("{} has no cx attribute", id))
        .parse()
        .expect(&format!("{} has invalid cx attribute", id));

    let y: f32 = attributes
        .get("cy")
        .expect(&format!("{} has no cy attribute", id))
        .parse()
        .expect(&format!("{} has invalid cy attribute", id));

    let position = Vec3::new(x, y, 0.01 * player_number as f32);

    let color_string = attributes
        .get("fill")
        .expect(&format!("{} has no fill attribute", id))
        .to_string();

    let color_hex_string = csscolorparser::parse(&color_string)
        .expect(&format!("{} has an invalid fill attribute", id))
        .to_hex_string();

    let color = Color::hex(color_hex_string).unwrap();

    commands.spawn(PlayerSpawn {
        player_number,
        position,
        color,
    });
}

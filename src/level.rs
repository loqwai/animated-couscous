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
            svg::parser::Event::Tag(path, tag_type, attributes) => {
                // ignore everything but empty tags for now. Non empty tags are containers, like
                // <g> and <svg>. We'll want to handle them at some point, but not yet.
                match tag_type {
                    svg::node::element::tag::Type::Empty => {}
                    _ => continue,
                }

                // player spawns are special, so handle them first
                if let Some(id) = attributes.get("id") {
                    if PLAYER_SPAWN_IDS.contains(&id.to_string().as_str()) {
                        handle_player_spawn(&mut commands, id, path, &attributes);
                        continue;
                    }
                }

                match path {
                    "rect" => spawn_rect(&mut commands, &mut meshes, &mut materials, &attributes),
                    _ => {
                        println!(
                            "ignored path {}, type: {:?}, id: {}, class: {}",
                            path,
                            tag_type,
                            get_string_value(&attributes, "id"),
                            get_string_value(&attributes, "class")
                        );
                    }
                };

                // if let Some(classes) = attributes.get("class") {
                //     let classes = classes.to_string();
                //     let classes: Vec<&str> = classes.split_whitespace().collect();
                // }

                // println!(
                //     "Found ignored tag {}, type: {:?}, id: {}, class: {}",
                //     path,
                //     tag_type,
                //     get_string_value(&attributes, "id"),
                //     get_string_value(&attributes, "class"),
                // );
            }
            _ => {}
        }
    }

    // commands.spawn(MaterialMesh2dBundle {
    //     mesh: meshes
    //         .add(shape::Quad::new(Vec2::new(1000., 1000.)).into())
    //         .into(),
    //     material: materials.add(ColorMaterial::from(Color::GRAY)),
    //     transform: Transform::from_translation(Vec3::new(0., -500., -0.1)),
    //     ..default()
    // });
}

fn get_string_value(attributes: &HashMap<String, svg::node::Value>, key: &str) -> String {
    match attributes.get(key) {
        Some(s) => s.to_string(),
        _ => return "<none>".to_string(),
    }
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

fn spawn_rect(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    attributes: &HashMap<String, svg::node::Value>,
) {
    let properties = parse_rect_properties(attributes);

    if let Err(e) = properties {
        println!(
            "Ignoring rect with invalid properties: {}, {:?}",
            e, attributes
        );
        return;
    }

    let (x, y, width, height) = properties.unwrap();

    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::Quad::new(Vec2::new(width, height)).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::GRAY)),
        transform: Transform::from_translation(Vec3::new(x, y, -0.1)),
        ..default()
    });
}

#[derive(Debug, Error)]
enum ParseRectError {
    /// Only numeric values allowed for "x". Percentages are not yet supported
    InvalidX,
    /// Only numeric values allowed for "y". Percentages are not yet supported
    InvalidY,
    /// Only numeric values allowed for "height". Percentages are not yet supported
    InvalidWidth,
    /// Only numeric values allowed for "width". Percentages are not yet supported
    InvalidHeight,
}

fn parse_rect_properties(
    attributes: &HashMap<String, svg::node::Value>,
) -> Result<(f32, f32, f32, f32), ParseRectError> {
    let x: f32 = attributes
        .get("x")
        .unwrap_or(&svg::node::Value::from("0"))
        .parse()
        .or(Err(ParseRectError::InvalidX))?;

    let y: f32 = attributes
        .get("y")
        .unwrap_or(&svg::node::Value::from("0"))
        .parse()
        .or(Err(ParseRectError::InvalidY))?;

    let width: f32 = attributes
        .get("width")
        .unwrap_or(&svg::node::Value::from("0"))
        .parse()
        .or(Err(ParseRectError::InvalidWidth))?;

    let height: f32 = attributes
        .get("height")
        .unwrap_or(&svg::node::Value::from("0"))
        .parse()
        .or(Err(ParseRectError::InvalidHeight))?;

    return Ok((x, y, width, height));
}

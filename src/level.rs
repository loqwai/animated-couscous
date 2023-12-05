use std::collections::HashMap;

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy_inspector_egui::egui::TextBuffer;

// const LEVEL_PATH: &str = "assets/level.svg";
const LEVEL_PATH: &str = "assets/plain.svg";

const PLAYER_SPAWN_IDS: [&str; 2] = ["player1Spawn", "player2Spawn"];

#[derive(Component)]
pub(crate) struct PlayerSpawn {
    pub player_number: u8,
    pub position: Vec3,
    pub color: Color,
}

#[derive(Debug, Error)]
pub(crate) enum LoadLevelError {
    HandleEmptyTagError(HandleEmptyTagError),
}

pub(crate) fn load_level<'a>(
    commands: Commands<'a, 'a>,
    meshes: ResMut<'a, Assets<Mesh>>,
    materials: ResMut<'a, Assets<ColorMaterial>>,
) -> Result<(), LoadLevelError> {
    let mut loader = Loader::new(commands, meshes, materials);
    loader.load_level(LEVEL_PATH.as_str())
}

struct Loader<'a> {
    commands: Commands<'a, 'a>,
    meshes: ResMut<'a, Assets<Mesh>>,
    materials: ResMut<'a, Assets<ColorMaterial>>,
}

impl<'a> Loader<'a> {
    fn new(
        commands: Commands<'a, 'a>,
        meshes: ResMut<'a, Assets<Mesh>>,
        materials: ResMut<'a, Assets<ColorMaterial>>,
    ) -> Self {
        Loader {
            commands,
            meshes,
            materials,
        }
    }

    fn load_level(&mut self, path: &str) -> Result<(), LoadLevelError> {
        let mut content = String::new();
        let parser = svg::open(path, &mut content).unwrap();
        for event in parser {
            match event {
                svg::parser::Event::Error(e) => panic!("Error parsing SVG: {:?}", e),
                svg::parser::Event::Tag(path, tag_type, attributes) => match tag_type {
                    svg::node::element::tag::Type::Start => {
                        self.handle_start_tag(path, &attributes)
                    }
                    svg::node::element::tag::Type::Empty => {
                        self.handle_empty_tag(path, &attributes)?
                    }
                    _ => continue,
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_start_tag(self: &Self, path: &str, attributes: &HashMap<String, svg::node::Value>) {
        match path {
            "svg" => {
                self.handle_svg_open();
            }
            _ => {
                println!(
                    "ignored path {}, id: {}, class: {}",
                    path,
                    get_string_debug_value(&attributes, "id"),
                    get_string_debug_value(&attributes, "class")
                );
            }
        }
    }

    fn handle_svg_open(self: &Self) {
        println!("svg open");
    }

    fn handle_empty_tag(
        self: &mut Self,
        path: &str,
        attributes: &HashMap<String, svg::node::Value>,
    ) -> Result<(), HandleEmptyTagError> {
        // player spawns are special, so handle them first
        if let Some(id) = attributes.get("id") {
            if PLAYER_SPAWN_IDS.contains(&id.to_string().as_str()) {
                self.handle_player_spawn(id, path, &attributes);
                return Ok(());
            }
        }

        match path {
            "rect" => self.handle_rect(&attributes)?,
            _ => {
                println!(
                    "ignored path {}, id: {}, class: {}",
                    path,
                    get_string_debug_value(&attributes, "id"),
                    get_string_debug_value(&attributes, "class")
                );
            }
        };

        Ok(())
    }

    fn handle_rect(
        self: &mut Self,
        attributes: &HashMap<String, svg::node::Value>,
    ) -> Result<(), ParseRectError> {
        let (x, y, width, height) = parse_rect_properties(attributes)?;

        self.commands.spawn(MaterialMesh2dBundle {
            mesh: self
                .meshes
                .add(shape::Quad::new(Vec2::new(width, height)).into())
                .into(),
            material: self.materials.add(ColorMaterial::from(Color::GRAY)),
            transform: Transform::from_translation(Vec3::new(x, y, -0.1)),
            ..default()
        });

        Ok(())
    }

    fn handle_player_spawn(
        self: &mut Self,
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

        self.commands.spawn(PlayerSpawn {
            player_number,
            position,
            color,
        });
    }
}

#[derive(Debug, Error)]
pub(crate) enum HandleEmptyTagError {
    ParseRectError(ParseRectError),
}

fn get_string_debug_value(attributes: &HashMap<String, svg::node::Value>, key: &str) -> String {
    match attributes.get(key) {
        Some(s) => s.to_string(),
        _ => return "<none>".to_string(),
    }
}

#[derive(Debug, Error)]
pub(crate) enum ParseRectError {
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

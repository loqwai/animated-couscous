mod view_box;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::sprite::MaterialMesh2dBundle;
use bevy_inspector_egui::egui::TextBuffer;
use bevy_rapier2d::prelude::*;

use self::view_box::ViewBox;

const LEVEL_PATH: &str = "assets/level.svg";
// const LEVEL_PATH: &str = "assets/plain.svg";
// const LEVEL_PATH: &str = "assets/half-plain.svg";
// const LEVEL_PATH: &str = "assets/offset-plain.svg";

const Z_SEPARATION: f32 = 0.01;

#[derive(Component)]
pub(crate) struct PlayerSpawn {
    pub player_number: u32,
    pub position: Vec3,
    pub color: Color,
    pub radius: f32,
}

#[derive(Bundle)]
pub(crate) struct ColliderBundle {
    body: RigidBody,
    collider: Collider,
}

#[derive(Debug, Error)]
pub(crate) enum LoadLevelError {
    HandleEmptyTagError(HandleEmptyTagError),
    HandleStartTagError(HandleStartTagError),
}

pub(crate) fn load_level<'a>(
    commands: Commands<'a, 'a>,
    meshes: ResMut<'a, Assets<Mesh>>,
    materials: ResMut<'a, Assets<ColorMaterial>>,
    window_width: f32,
    window_height: f32,
) -> Result<(), LoadLevelError> {
    let mut loader = Loader::new(commands, meshes, materials, window_width, window_height);
    loader.load_level(LEVEL_PATH.as_str())
}

struct Loader<'a> {
    commands: Commands<'a, 'a>,
    meshes: ResMut<'a, Assets<Mesh>>,
    materials: ResMut<'a, Assets<ColorMaterial>>,
    window_width: f32,
    window_height: f32,
    current_z: f32,
    view_box: Option<ViewBox>,
}

impl<'a> Loader<'a> {
    fn new(
        commands: Commands<'a, 'a>,
        meshes: ResMut<'a, Assets<Mesh>>,
        materials: ResMut<'a, Assets<ColorMaterial>>,
        window_width: f32,
        window_height: f32,
    ) -> Self {
        Loader {
            commands,
            meshes,
            materials,
            window_width,
            window_height,
            current_z: 0.0,
            view_box: None,
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
                        self.handle_start_tag(path, &attributes)?
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

    fn handle_start_tag(
        self: &mut Self,
        path: &str,
        attributes: &HashMap<String, svg::node::Value>,
    ) -> Result<(), HandleStartTagError> {
        match path {
            "svg" => self.handle_svg_open(attributes)?,
            _ => {
                println!(
                    "ignored path {}, id: {}, class: {}",
                    path,
                    get_string_debug_value(&attributes, "id"),
                    get_string_debug_value(&attributes, "class")
                );
            }
        }

        Ok(())
    }

    fn handle_empty_tag(
        self: &mut Self,
        path: &str,
        attributes: &HashMap<String, svg::node::Value>,
    ) -> Result<(), HandleEmptyTagError> {
        match path {
            "circle" => self.handle_circle(&attributes)?,
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

    fn handle_svg_open(
        self: &mut Self,
        attributes: &HashMap<String, svg::node::Value>,
    ) -> Result<(), HandleSvgOpenError> {
        let view_box: ViewBox = attributes
            .get("viewBox")
            .ok_or(HandleSvgOpenError::MissingViewBox)?
            .parse()?;

        self.view_box = Some(view_box);

        Ok(())
    }

    fn handle_rect(
        self: &mut Self,
        attributes: &HashMap<String, svg::node::Value>,
    ) -> Result<(), HandleRectError> {
        let (x, y, width, height) = parse_rect_properties(attributes)?;

        let z = self.current_z;
        self.current_z += Z_SEPARATION;

        let x = self.adjusted_x(x, width)?;
        let y = self.adjusted_y(y, height)?;

        let width = self.adjusted_width(width)?;
        let height = self.adjusted_height(height)?;

        let fill_string = attributes
            .get("fill")
            .unwrap_or(&svg::node::Value::from("rgba(0,0,0,0)"))
            .to_string();

        let fill = parse_color(&fill_string)?;

        let entity = self
            .commands
            .spawn(MaterialMesh2dBundle {
                mesh: self
                    .meshes
                    .add(shape::Quad::new(Vec2::new(width, height)).into())
                    .into(),
                material: self.materials.add(ColorMaterial::from(fill)),
                transform: Transform::from_translation(Vec3::new(x, y, z)),
                ..default()
            })
            .id();

        if has_class(attributes, "collider") {
            self.commands.entity(entity).insert(ColliderBundle {
                body: RigidBody::Fixed,
                collider: Collider::cuboid(width / 2., height / 2.),
            });
        }

        Ok(())
    }

    fn handle_circle(
        self: &mut Self,
        attributes: &HashMap<String, svg::node::Value>,
    ) -> Result<(), HandleCircleError> {
        if has_class(attributes, "spawn-player") {
            self.handle_player_spawn(attributes)?;
            return Ok(());
        }

        let z = self.current_z;
        self.current_z += Z_SEPARATION;

        let r: f32 = attributes
            .get("r")
            .unwrap_or(&svg::node::Value::from("0"))
            .parse()
            .or(Err(HandleCircleError::InvalidR))?;
        let radius = self.adjusted_width(r * 2.)? / 2.;

        let x: f32 = attributes
            .get("cx")
            .unwrap_or(&svg::node::Value::from("0"))
            .parse()
            .or(Err(HandleCircleError::InvalidCx))?;
        let x = self.adjusted_x(x, r * 2.)? + radius;

        let y: f32 = attributes
            .get("cy")
            .unwrap_or(&svg::node::Value::from("0"))
            .parse()
            .or(Err(HandleCircleError::InvalidCy))?;
        let y = self.adjusted_y(y, r * 2.)? + radius;

        let position = Vec3::new(x, y, z);

        let color_string = attributes
            .get("fill")
            .ok_or(HandleCircleError::MissingFill)?
            .to_string();

        let color = parse_color(&color_string)?;

        let entity = self
            .commands
            .spawn(MaterialMesh2dBundle {
                mesh: self.meshes.add(shape::Circle::new(radius).into()).into(),
                material: self.materials.add(ColorMaterial::from(color)),
                transform: Transform::from_translation(position),
                ..default()
            })
            .id();

        if has_class(attributes, "collider") {
            self.commands.entity(entity).insert(ColliderBundle {
                body: RigidBody::Fixed,
                collider: Collider::ball(radius),
            });
        }

        Ok(())
    }

    fn handle_player_spawn(
        self: &mut Self,
        attributes: &HashMap<String, svg::node::Value>,
    ) -> Result<(), HandlePlayerSpawnError> {
        let z = self.current_z;
        self.current_z += Z_SEPARATION;

        let player_number: u32 = attributes
            .get("data-player-number")
            .ok_or(HandlePlayerSpawnError::MissingPlayerNumber)?
            .parse()
            .or(Err(HandlePlayerSpawnError::InvalidPlayerNumber))?;

        let r: f32 = attributes
            .get("r")
            .unwrap_or(&svg::node::Value::from("0"))
            .parse()
            .or(Err(HandlePlayerSpawnError::InvalidR))?;
        let radius = self.adjusted_width(r * 2.)? / 2.;

        let x: f32 = attributes
            .get("cx")
            .unwrap_or(&svg::node::Value::from("0"))
            .parse()
            .or(Err(HandlePlayerSpawnError::InvalidCx))?;
        let x = self.adjusted_x(x, r * 2.)? + radius;

        let y: f32 = attributes
            .get("cy")
            .unwrap_or(&svg::node::Value::from("0"))
            .parse()
            .or(Err(HandlePlayerSpawnError::InvalidCy))?;
        let y = self.adjusted_y(y, r * 2.)? + radius;

        let position = Vec3::new(x, y, z);

        let color_string = attributes
            .get("fill")
            .ok_or(HandlePlayerSpawnError::MissingFill)?
            .to_string();

        let color = parse_color(&color_string)?;

        self.commands.spawn(PlayerSpawn {
            player_number,
            position,
            color,
            radius,
        });

        Ok(())
    }

    fn adjusted_x(self: &Self, x: f32, width: f32) -> Result<f32, AdjustmentError> {
        let view_box = self.view_box.ok_or(AdjustmentError::MissingViewBox)?;

        let x = x - view_box.x; // adjust x so that it represents the distance from the left edge of the view box
        let x = x + (width / 2.0); // adjust x so that it represents the center of the rec instead of the left edge
        let x = (x / view_box.width) * 2.0 - 1.0; // normalize x so that it is between -1 and 1
        let x = x * (self.window_width / 2.0); // adjust x so that it is between (-window_width / 2) & (window_width / 2)

        Ok(x)
    }

    fn adjusted_y(self: &Self, y: f32, height: f32) -> Result<f32, AdjustmentError> {
        let view_box = self.view_box.ok_or(AdjustmentError::MissingViewBox)?;

        let y = y - view_box.y; // adjust y so that it represents the distance from the top edge of the view box
        let y = y + (height / 2.0); // adjust y so that it represents the center of the rec instead of the top edge
        let y = (y / view_box.height) * 2.0 - 1.0; // normalize y so that it is between -1 and 1
        let y = -y; // invert y so that it is between 1 and -1 (SVG is top-down, Bevy is bottom-up)
        let y = y * (self.window_height / 2.0); // adjust y so that it is between (-window_height / 2) & (window_height / 2)

        Ok(y)
    }

    fn adjusted_width(self: &Self, width: f32) -> Result<f32, AdjustmentError> {
        let view_box = self.view_box.ok_or(AdjustmentError::MissingViewBox)?;

        let width = width / view_box.width; // adjust width so that it is between 0 & 1
        let width = width * self.window_width; // adjust width so that it is between 0 & window_width

        Ok(width)
    }

    fn adjusted_height(self: &Self, height: f32) -> Result<f32, AdjustmentError> {
        let view_box = self.view_box.ok_or(AdjustmentError::MissingViewBox)?;

        let height = height / view_box.height; // adjust height so that it is between 0 & 1
        let height = height * self.window_height; // adjust height so that it is between 0 & window_height

        Ok(height)
    }
}

#[derive(Debug, Error)]
pub(crate) enum HandleStartTagError {
    HandleSvgOpenError(HandleSvgOpenError),
}

#[derive(Debug, Error)]
pub(crate) enum HandleEmptyTagError {
    HandleRectError(HandleRectError),
    HandleCircleError(HandleCircleError),
}

#[derive(Debug, Error)]
pub(crate) enum HandleRectError {
    ParseRectError(ParseRectError),
    InvalidFill(csscolorparser::ParseColorError),
    AdjustmentError(AdjustmentError),
}

#[derive(Debug, Error)]
pub(crate) enum HandleCircleError {
    InvalidCx,
    InvalidCy,
    InvalidR,
    MissingFill,
    AdjustmentError(AdjustmentError),
    InvalidFill(csscolorparser::ParseColorError),
    HandlePlayerSpawnError(HandlePlayerSpawnError),
}

#[derive(Debug, Error)]
pub(crate) enum HandlePlayerSpawnError {
    InvalidCx,
    InvalidCy,
    InvalidR,
    MissingFill,
    InvalidPlayerNumber,
    MissingPlayerNumber,
    AdjustmentError(AdjustmentError),
    InvalidFill(csscolorparser::ParseColorError),
}

#[derive(Debug, Error)]
pub(crate) enum AdjustmentError {
    MissingViewBox,
}

fn get_string_debug_value(attributes: &HashMap<String, svg::node::Value>, key: &str) -> String {
    match attributes.get(key) {
        Some(s) => s.to_string(),
        _ => return "<none>".to_string(),
    }
}

#[derive(Debug, Error)]
pub(crate) enum HandleSvgOpenError {
    /// viewBox attribute is required
    MissingViewBox,
    /// viewBox attribute is invalid. Must be "x y width height", and be all numeric
    InvalidViewBox(view_box::ParseViewBoxError),
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

fn parse_color(color: &str) -> Result<Color, csscolorparser::ParseColorError> {
    let (r, g, b, a) = csscolorparser::parse(color)?.to_linear_rgba();
    Ok(Color::rgba(r as f32, g as f32, b as f32, a as f32))
}

fn has_class(attributes: &HashMap<String, svg::node::Value>, needle: &str) -> bool {
    match attributes.get("class") {
        Some(value) => {
            for class in value.to_string().split_whitespace() {
                if class == needle {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

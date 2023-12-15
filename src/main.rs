#[macro_use]
extern crate derive_error;

// mod game_state;
mod events;
mod input;
mod level;
mod manage_state;
mod render;

use bevy::prelude::*;
use bevy::window::WindowPlugin;
use bevy::window::WindowResolution;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use bevy_rapier2d::render::RapierDebugRenderPlugin;
use input::InputPlugin;
// use game_state::GameStateEvent;
use manage_state::ManageStatePlugin;

use render::RenderPlugin;
use uuid::Uuid;

fn main() {
    let window_offset: i32 = std::env::var("WINDOW_OFFSET")
        .unwrap_or("0".to_string())
        .parse()
        .unwrap();

    let width: f32 = 1000.;
    let height: f32 = 400.;

    App::new()
        .insert_resource(AppConfig {
            width,
            height,

            client_id: Uuid::new_v4().to_string(),
            // not implemented yet
            fudge_factor: 1.,
            bullet_speed: 1000.,
            player_move_speed: 400.,
            // fire_timeout: 500,
            jump_amount: 50.,
            gravity: 2000.,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(width, height),
                position: WindowPosition::new(IVec2 {
                    x: 0,
                    y: window_offset,
                }),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(RenderPlugin)
        .add_plugins(ManageStatePlugin)
        .add_plugins(InputPlugin)
        // .add_event::<GameStateEvent>()
        .run();
}

#[derive(Resource)]
struct AppConfig {
    width: f32,
    height: f32,

    client_id: String,
    /// How much to displace the bullet from the player so
    /// they don't shoot themselves if they're running towards
    /// where they're shooting
    fudge_factor: f32,

    bullet_speed: f32,
    player_move_speed: f32,
    // fire_timeout: u64,
    jump_amount: f32,
    gravity: f32,
}

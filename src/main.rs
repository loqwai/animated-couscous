#[macro_use]
extern crate derive_error;

// mod game_state;
mod events;
mod input;
mod level;
mod manage_state;
mod render;

mod client;
mod protos;
mod server;

use bevy::prelude::*;
use bevy::window::WindowPlugin;
use bevy::window::WindowResolution;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use client::ClientPlugin;
use input::InputPlugin;
// use game_state::GameStateEvent;
use manage_state::ManageStatePlugin;

use render::RenderPlugin;
use server::ServerPlugin;
use uuid::Uuid;

fn main() {
    let window_offset: i32 = std::env::var("WINDOW_OFFSET")
        .unwrap_or("0".to_string())
        .parse()
        .unwrap();

    let width: f32 = 1000.;
    let height: f32 = 400.;

    let enable_physics: bool = std::env::var("ENABLE_PHYSICS")
            .unwrap_or("true".to_string())
            .parse()
            .expect("Failed to parse boolean value for ENABLE_PHYSICS. Accepted values are 'true' or 'false'");

    let mut app = App::new();
    app.insert_resource(AppConfig {
        width,
        height,

        client_id: Uuid::new_v4().to_string(),
        // not implemented yet
        fudge_factor: 11.,
        bullet_speed: 1000.,
        player_move_speed: 80.,
        reload_timeout: 1000,
        jump_amount: 400.,
        gravity: 2000.,
        player_max_move_speed: 500.,

        shield_timeout: 1000,
        shield_duration: 500,
    })
    .register_type::<AppConfig>()
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
    .add_plugins(RenderPlugin)
    .add_plugins(InputPlugin)
    .add_plugins(ManageStatePlugin::with_physics(enable_physics));

    if let Ok(hostname) = std::env::var("SERVE_ON") {
        app.add_plugins(ServerPlugin::serve_on(hostname));
    }

    if let Ok(hostname) = std::env::var("CONNECT_TO") {
        app.add_plugins(ClientPlugin::connect_to(hostname));
    }

    app.run();
}

#[derive(Resource, Reflect)]
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
    reload_timeout: u64,
    jump_amount: f32,
    gravity: f32,

    shield_timeout: u64,
    shield_duration: u64,
    player_max_move_speed: f32,
}

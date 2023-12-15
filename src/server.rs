use std::{
    net::{TcpListener, TcpStream},
    thread,
};

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use protobuf::Message;

use crate::{
    manage_state::{Bullet, Player},
    protos::generated::applesauce,
};

pub(crate) struct ServerPlugin {
    hostname: String,
}

impl ServerPlugin {
    pub(crate) fn serve_on(hostname: String) -> Self {
        Self { hostname }
    }
}

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerConfig {
            hostname: self.hostname.clone(),
        })
        .add_systems(Startup, serve)
        .add_systems(PostUpdate, send_state);
    }
}

#[derive(Resource)]
struct ServerConfig {
    hostname: String,
}

// #[derive(Resource)]
// struct InputEventReceiver(Receiver<applesauce::GameState>);

#[derive(Resource)]
struct GameStateSender(Sender<applesauce::GameState>);

fn serve(mut commands: Commands, config: Res<ServerConfig>) {
    let listener = TcpListener::bind(config.hostname.clone()).unwrap();

    let (tx_game_state, rx_game_state) = crossbeam_channel::unbounded::<applesauce::GameState>();
    commands.insert_resource(GameStateSender(tx_game_state));

    // let (tx_outgoing, rx_outgoing) = crossbeam_channel::unbounded::<applesauce::GameState>();
    // commands.insert_resource(InputEventReceiver(rx_outgoing));

    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let rx_game_state = rx_game_state.clone();

            thread::spawn(move || forward_game_state_to_stream(stream, rx_game_state));
        }
    });
}

fn forward_game_state_to_stream(
    mut stream: TcpStream,
    rx_game_state: Receiver<applesauce::GameState>,
) {
    for game_state in rx_game_state.iter() {
        game_state
            .write_length_delimited_to_writer(&mut stream)
            .unwrap();
    }
}

fn send_state(
    sender: Res<GameStateSender>,
    players: Query<(&Player, &Transform)>,
    bullets: Query<(&Bullet, &Transform)>,
    time: Res<Time>,
) {
    sender
        .0
        .send(applesauce::GameState {
            timestamp: time.elapsed().as_millis() as u64,
            players: players
                .iter()
                .map(|(player, transform)| applesauce::Player {
                    id: player.id.to_string(),
                    client_id: player.client_id.to_string(),
                    spawn_id: player.spawn_id.to_string(),
                    radius: player.radius,
                    color: applesauce::Color::from(player.color).into(),
                    position: applesauce::Vec3::from(transform.translation.clone()).into(),
                    special_fields: default(),
                })
                .collect(),
            bullets: bullets
                .iter()
                .map(|(bullet, transform)| applesauce::Bullet {
                    id: bullet.id.to_string(),
                    position: applesauce::Vec3::from(transform.translation.clone()).into(),
                    special_fields: default(),
                })
                .collect(),
            special_fields: default(),
        })
        .unwrap();
}

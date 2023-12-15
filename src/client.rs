use std::{net::TcpStream, thread};

use bevy::prelude::*;
use crossbeam_channel::Receiver;
use protobuf::CodedInputStream;

use crate::{
    manage_state::{Player, PlayerBundle},
    protos::generated::applesauce,
};

pub(crate) struct ClientPlugin {
    hostname: String,
}

impl ClientPlugin {
    pub(crate) fn connect_to(hostname: String) -> Self {
        Self { hostname }
    }
}

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClientConfig {
            hostname: self.hostname.clone(),
        })
        .add_systems(Startup, connect_to_server)
        .add_systems(Update, update_game_state_from_network);
    }
}

#[derive(Resource)]
struct ClientConfig {
    hostname: String,
}

#[derive(Resource)]
struct ReceiveGameState(Receiver<applesauce::GameState>);

fn connect_to_server(mut commands: Commands, config: Res<ClientConfig>) {
    let stream = TcpStream::connect(config.hostname.clone()).unwrap();
    let (tx_game_state, rx_game_state) = crossbeam_channel::unbounded::<applesauce::GameState>();

    commands.insert_resource(ReceiveGameState(rx_game_state));

    thread::spawn(move || {
        let mut stream = stream.try_clone().unwrap();
        let mut stream = CodedInputStream::new(&mut stream);

        loop {
            if stream.eof().unwrap() {
                break;
            }

            let game_state: applesauce::GameState = stream.read_message().unwrap();
            tx_game_state.send(game_state).unwrap();
        }
    });
}

fn update_game_state_from_network(
    mut commands: Commands,
    receiver: Res<ReceiveGameState>,
    mut players: Query<(&mut Player, &mut Transform)>,
) {
    match receiver
        .0
        .try_iter()
        .max_by(|a, b| a.timestamp.cmp(&b.timestamp))
    {
        None => return,
        Some(game_state) => {
            for player_state in game_state.players.iter() {
                match players.iter_mut().find(|(p, _)| p.id == player_state.id) {
                    None => {
                        commands.spawn(PlayerBundle::new(
                            Player {
                                id: player_state.id.clone(),
                                client_id: player_state.client_id.clone(),
                                spawn_id: player_state.spawn_id.clone(),
                                radius: player_state.radius,
                                color: player_state.color.clone().unwrap().into(),
                            },
                            Transform::from_translation(
                                player_state.position.clone().unwrap().into(),
                            ),
                        ));
                    }
                    Some((mut player, mut transform)) => {
                        player.client_id = player_state.client_id.clone();
                        player.spawn_id = player_state.spawn_id.clone();
                        player.radius = player_state.radius;
                        player.color = player_state.color.clone().unwrap().into();
                        transform.translation = player_state.position.clone().unwrap().into();
                    }
                }
            }
        }
    };
}

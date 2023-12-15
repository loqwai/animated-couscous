use std::{net::TcpStream, thread};

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use protobuf::{CodedInputStream, Message};

use crate::{
    events::{
        PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent, PlayerShootEvent,
        PlayerSpawnEvent,
    },
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
        .add_systems(
            Update,
            (update_game_state_from_network, write_inputs_to_network),
        );
    }
}

#[derive(Resource)]
struct ClientConfig {
    hostname: String,
}

#[derive(Resource)]
struct ReceiveGameState(Receiver<applesauce::GameState>);

#[derive(Resource)]
struct SendInput(Sender<applesauce::Input>);

fn connect_to_server(mut commands: Commands, config: Res<ClientConfig>) {
    let stream = TcpStream::connect(config.hostname.clone()).unwrap();
    let (tx_game_state, rx_game_state) = crossbeam_channel::unbounded::<applesauce::GameState>();
    let (tx_input, rx_input) = crossbeam_channel::unbounded::<applesauce::Input>();

    commands.insert_resource(ReceiveGameState(rx_game_state));
    commands.insert_resource(SendInput(tx_input));

    let mut recv_stream = stream.try_clone().unwrap();
    thread::spawn(move || {
        let mut coded_stream = CodedInputStream::new(&mut recv_stream);

        loop {
            if coded_stream.eof().unwrap() {
                break;
            }

            let game_state: applesauce::GameState = coded_stream.read_message().unwrap();
            tx_game_state.send(game_state).unwrap();
        }
    });

    let mut send_stream = stream.try_clone().unwrap();
    thread::spawn(move || loop {
        let input = rx_input.recv().unwrap();
        input
            .write_length_delimited_to_writer(&mut send_stream)
            .unwrap();
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

fn write_inputs_to_network(
    sender: Res<SendInput>,
    mut spawn_events: EventReader<PlayerSpawnEvent>,
    mut move_left_events: EventReader<PlayerMoveLeftEvent>,
    mut move_right_events: EventReader<PlayerMoveRightEvent>,
    mut jump_events: EventReader<PlayerJumpEvent>,
    mut shoot_events: EventReader<PlayerShootEvent>,
) {
    for event in spawn_events.read() {
        sender.0.send(event.into()).unwrap();
    }

    for event in move_left_events.read() {
        sender.0.send(event.into()).unwrap();
    }

    for event in move_right_events.read() {
        sender.0.send(event.into()).unwrap();
    }

    for event in jump_events.read() {
        sender.0.send(event.into()).unwrap();
    }

    for event in shoot_events.read() {
        sender.0.send(event.into()).unwrap();
    }
}

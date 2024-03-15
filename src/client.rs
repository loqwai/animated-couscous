use std::{net::TcpStream, thread};

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use protobuf::{CodedInputStream, Message};

use crate::{
    events::{
        PlayerBlockEvent, PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent,
        PlayerShootEvent, PlayerSpawnEvent,
    },
    manage_state::GameStateEvent,
    protos::generated::applesauce,
    AppConfig,
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
        .insert_resource(LatestEventTime(None))
        .add_systems(Startup, connect_to_server)
        .add_event::<GameStateEvent>()
        .add_systems(
            Update,
            (
                proxy_game_state_from_network,
                write_inputs_to_network,
                update_client_id_from_identity,
            ),
        );
    }
}

#[derive(Component, Reflect)]
struct Despawn;

#[derive(Resource)]
struct ClientConfig {
    hostname: String,
}

#[derive(Resource, Deref, DerefMut)]
struct LatestEventTime(Option<u64>);

#[derive(Resource, Deref)]
struct ReceiveGameState(Receiver<applesauce::GameState>);

#[derive(Resource, Deref)]
struct ReceiveIdentity(Receiver<applesauce::Identity>);

#[derive(Resource, Deref)]
struct SendInput(Sender<applesauce::Input>);

fn connect_to_server(mut commands: Commands, config: Res<ClientConfig>) {
    let stream = TcpStream::connect(config.hostname.clone()).unwrap();
    let (tx_identity, rx_identity) = crossbeam_channel::unbounded::<applesauce::Identity>();
    let (tx_game_state, rx_game_state) = crossbeam_channel::unbounded::<applesauce::GameState>();
    let (tx_input, rx_input) = crossbeam_channel::unbounded::<applesauce::Input>();

    commands.insert_resource(ReceiveIdentity(rx_identity));
    commands.insert_resource(ReceiveGameState(rx_game_state));
    commands.insert_resource(SendInput(tx_input));

    let mut recv_stream = stream.try_clone().unwrap();
    thread::spawn(move || {
        let mut coded_stream = CodedInputStream::new(&mut recv_stream);

        // the first message is an Identity message letting us know what our client id is
        let identity: applesauce::Identity = coded_stream.read_message().unwrap();
        tx_identity.send(identity).unwrap();

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

fn update_client_id_from_identity(
    identity: Res<ReceiveIdentity>,
    mut app_config: ResMut<AppConfig>,
) {
    match identity.try_iter().next() {
        None => return,
        Some(identity) => {
            app_config.client_id = identity.client_id;
        }
    }
}

fn proxy_game_state_from_network(
    receiver: Res<ReceiveGameState>,
    mut events: EventWriter<GameStateEvent>,
    mut latest_event_time: ResMut<LatestEventTime>,
) {
    match receiver
        .try_iter()
        .max_by(|a, b| a.timestamp.cmp(&b.timestamp))
    {
        None => return,
        Some(game_state) => {
            if game_state.timestamp <= *latest_event_time.get_or_insert(0) {
                return;
            }
            latest_event_time.replace(game_state.timestamp);
            events.send(game_state.into());
        }
    }
}

fn write_inputs_to_network(
    sender: Res<SendInput>,
    mut spawn_events: EventReader<PlayerSpawnEvent>,
    mut move_left_events: EventReader<PlayerMoveLeftEvent>,
    mut move_right_events: EventReader<PlayerMoveRightEvent>,
    mut jump_events: EventReader<PlayerJumpEvent>,
    mut shoot_events: EventReader<PlayerShootEvent>,
    mut block_events: EventReader<PlayerBlockEvent>,
) {
    for event in spawn_events.read() {
        sender.send(event.into()).unwrap();
    }

    for event in move_left_events.read() {
        sender.send(event.into()).unwrap();
    }

    for event in move_right_events.read() {
        sender.send(event.into()).unwrap();
    }

    for event in jump_events.read() {
        sender.send(event.into()).unwrap();
    }

    for event in shoot_events.read() {
        sender.send(event.into()).unwrap();
    }

    for event in block_events.read() {
        sender.send(event.into()).unwrap();
    }
}

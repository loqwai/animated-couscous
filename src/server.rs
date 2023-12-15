use std::{
    net::{TcpListener, TcpStream},
    thread,
};

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use protobuf::{CodedInputStream, Message};

use crate::{
    events::{
        PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent, PlayerShootEvent,
        PlayerSpawnEvent,
    },
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
        .add_systems(PreUpdate, recv_input)
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

#[derive(Resource)]
struct InputReceiver(Receiver<applesauce::Input>);

fn serve(mut commands: Commands, config: Res<ServerConfig>) {
    let listener = TcpListener::bind(config.hostname.clone()).unwrap();

    let (tx_game_state, rx_game_state) = crossbeam_channel::bounded::<applesauce::GameState>(1);
    commands.insert_resource(GameStateSender(tx_game_state));

    let (tx_input, rx_input) = crossbeam_channel::bounded::<applesauce::Input>(1);
    commands.insert_resource(InputReceiver(rx_input));

    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();

            let send_stream = stream.try_clone().unwrap();
            let rx_game_state = rx_game_state.clone();

            let recv_stream = stream.try_clone().unwrap();
            let tx_input = tx_input.clone();

            thread::spawn(move || forward_game_state_to_stream(send_stream, rx_game_state));
            thread::spawn(move || read_network_input_events(recv_stream, tx_input));
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

fn read_network_input_events(mut stream: TcpStream, tx_input: Sender<applesauce::Input>) {
    let mut coded_stream = CodedInputStream::new(&mut stream);

    loop {
        if coded_stream.eof().unwrap() {
            break;
        }

        let input: applesauce::Input = coded_stream.read_message().unwrap();
        tx_input.send(input).unwrap();
    }
}

fn recv_input(
    receiver: Res<InputReceiver>,
    mut spawn_events: EventWriter<PlayerSpawnEvent>,
    mut move_left_events: EventWriter<PlayerMoveLeftEvent>,
    mut move_right_events: EventWriter<PlayerMoveRightEvent>,
    mut jump_events: EventWriter<PlayerJumpEvent>,
    mut shoot_events: EventWriter<PlayerShootEvent>,
) {
    receiver.0.try_iter().for_each(|input| match input.inner {
        Some(applesauce::input::Inner::Spawn(_)) => spawn_events.send(PlayerSpawnEvent {
            client_id: input.client_id,
        }),
        Some(applesauce::input::Inner::MoveLeft(_)) => move_left_events.send(PlayerMoveLeftEvent {
            client_id: input.client_id,
        }),
        Some(applesauce::input::Inner::MoveRight(_)) => {
            move_right_events.send(PlayerMoveRightEvent {
                client_id: input.client_id,
            })
        }
        Some(applesauce::input::Inner::Jump(_)) => jump_events.send(PlayerJumpEvent {
            client_id: input.client_id,
        }),
        Some(applesauce::input::Inner::Shoot(shoot)) => shoot_events.send(PlayerShootEvent {
            client_id: input.client_id,
            aim: shoot.aim.unwrap().into(),
        }),
        None => {}
    });
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
                    rotation: applesauce::Quat::from(transform.rotation.clone()).into(),
                    special_fields: default(),
                })
                .collect(),
            special_fields: default(),
        })
        .unwrap();
}

use std::{
    net::{TcpListener, TcpStream},
    thread,
};

use bevy::prelude::*;
use bevy_rapier2d::dynamics::Velocity;
use crossbeam_channel::{Receiver, Sender};
use protobuf::{CodedInputStream, Message};
use uuid::Uuid;

use crate::{
    events::{
        PlayerBlockEvent, PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent,
        PlayerShootEvent, PlayerSpawnEvent,
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
        .add_systems(PreUpdate, handle_identity)
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

#[derive(Resource, Deref)]
struct GameStateSender(Sender<applesauce::GameState>);

#[derive(Resource, Deref)]
struct InputReceiver(Receiver<applesauce::Input>);

#[derive(Resource, Deref)]
struct IdentityReceiver(Receiver<applesauce::Identity>);

fn serve(mut commands: Commands, config: Res<ServerConfig>) {
    let listener = TcpListener::bind(config.hostname.clone()).unwrap();

    let (tx_game_state, rx_game_state) = crossbeam_channel::unbounded::<applesauce::GameState>();
    commands.insert_resource(GameStateSender(tx_game_state));

    let (tx_input, rx_input) = crossbeam_channel::unbounded::<applesauce::Input>();
    commands.insert_resource(InputReceiver(rx_input));

    let (tx_identity, rx_identity) = crossbeam_channel::unbounded::<applesauce::Identity>();
    commands.insert_resource(IdentityReceiver(rx_identity));

    let (tx_stream, rx_stream) = crossbeam_channel::unbounded::<TcpStream>();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();

            let recv_stream = stream.try_clone().unwrap();
            let tx_input = tx_input.clone();

            tx_stream.send(stream.try_clone().unwrap()).unwrap();
            thread::spawn(move || read_network_input_events(recv_stream, tx_input));

            let identity = applesauce::Identity {
                client_id: Uuid::new_v4().to_string(),
                ..Default::default()
            };

            tx_identity.send(identity.clone()).unwrap();

            identity
                .write_length_delimited_to_writer(&mut stream)
                .unwrap();
        }
    });

    thread::spawn(move || {
        let mut streams: Vec<TcpStream> = vec![];

        for game_state in rx_game_state.iter() {
            for stream in rx_stream.try_iter() {
                streams.push(stream);
            }

            for mut stream in streams.iter() {
                game_state
                    .write_length_delimited_to_writer(&mut stream)
                    .unwrap();
            }
        }
    });
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
    mut block_events: EventWriter<PlayerBlockEvent>,
) {
    receiver.try_iter().for_each(|input| match input.inner {
        Some(applesauce::input::Inner::Spawn(_)) => spawn_events.send(PlayerSpawnEvent {
            id: input.id,
            client_id: input.client_id,
        }),
        Some(applesauce::input::Inner::MoveLeft(_)) => move_left_events.send(PlayerMoveLeftEvent {
            id: input.id,
            client_id: input.client_id,
        }),
        Some(applesauce::input::Inner::MoveRight(_)) => {
            move_right_events.send(PlayerMoveRightEvent {
                id: input.id,
                client_id: input.client_id,
            })
        }
        Some(applesauce::input::Inner::Jump(_)) => jump_events.send(PlayerJumpEvent {
            id: input.id,
            client_id: input.client_id,
        }),
        Some(applesauce::input::Inner::Shoot(shoot)) => shoot_events.send(PlayerShootEvent {
            id: input.id,
            client_id: input.client_id,
            aim: shoot.aim.unwrap().into(),
        }),
        Some(applesauce::input::Inner::Block(_)) => block_events.send(PlayerBlockEvent {
            id: input.id,
            client_id: input.client_id,
        }),
        None => {}
    });
}

fn send_state(
    sender: Res<GameStateSender>,
    players: Query<(&Player, &Transform, &Velocity)>,
    bullets: Query<(&Bullet, &Transform, &Velocity)>,
    time: Res<Time>,
) {
    sender
        .send(applesauce::GameState {
            timestamp: time.elapsed().as_millis() as u64,
            players: players
                .iter()
                .map(|(player, transform, velocity)| applesauce::Player {
                    id: player.id.to_string(),
                    client_id: player.client_id.to_string(),
                    spawn_id: player.spawn_id.to_string(),
                    radius: player.radius,
                    color: applesauce::Color::from(player.color).into(),
                    position: applesauce::Vec3::from(transform.translation).into(),
                    velocity: applesauce::Vec2::from(velocity.linvel).into(),
                    special_fields: default(),
                })
                .collect(),
            bullets: bullets
                .iter()
                .map(|(bullet, transform, velocity)| applesauce::Bullet {
                    id: bullet.id.to_string(),
                    position: applesauce::Vec3::from(transform.translation).into(),
                    rotation: applesauce::Quat::from(transform.rotation).into(),
                    velocity: applesauce::Vec2::from(velocity.linvel).into(),
                    special_fields: default(),
                })
                .collect(),
            special_fields: default(),
        })
        .unwrap();
}

fn handle_identity(mut commands: Commands, receiver: Res<IdentityReceiver>) {
    receiver.try_iter().for_each(|identity| {
        commands.spawn(crate::Player {
            client_id: identity.client_id,
        });
    })
}

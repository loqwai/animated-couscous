use std::{net::TcpStream, thread};

use bevy::{prelude::*, utils::HashMap};
use crossbeam_channel::{Receiver, Sender};
use protobuf::{CodedInputStream, Message};

use crate::{
    events::{
        PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent, PlayerShootEvent,
        PlayerSpawnEvent,
    },
    level,
    manage_state::{Bullet, Player},
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
        .add_systems(Startup, (load_level, connect_to_server))
        .add_systems(
            Update,
            (update_game_state_from_network, write_inputs_to_network),
        )
        .add_systems(PostUpdate, despawn_things_that_need_despawning);
    }
}

#[derive(Component, Reflect)]
struct Despawn;

#[derive(Resource)]
struct ClientConfig {
    hostname: String,
}

#[derive(Resource)]
struct ReceiveGameState(Receiver<applesauce::GameState>);

#[derive(Resource)]
struct SendInput(Sender<applesauce::Input>);

fn load_level(
    commands: Commands,
    config: Res<AppConfig>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    level::load_level(commands, meshes, materials, config.width, config.height)
        .expect("Failed to load level");
}

fn connect_to_server(mut commands: Commands, config: Res<ClientConfig>) {
    let stream = TcpStream::connect(config.hostname.clone()).unwrap();
    let (tx_game_state, rx_game_state) = crossbeam_channel::bounded::<applesauce::GameState>(10);
    let (tx_input, rx_input) = crossbeam_channel::bounded::<applesauce::Input>(10);

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
    players: Query<(Entity, &mut Player, &mut Transform), Without<Bullet>>,
    bullets: Query<(Entity, &mut Bullet, &mut Transform), Without<Player>>,
) {
    match receiver
        .0
        .try_iter()
        .max_by(|a, b| a.timestamp.cmp(&b.timestamp))
    {
        None => return,
        Some(game_state) => {
            sync_bullets(&game_state, bullets, &mut commands);
            sync_players(&game_state, players, &mut commands);
        }
    }
}

fn sync_bullets(
    game_state: &applesauce::GameState,
    mut bullets: Query<'_, '_, (Entity, &mut Bullet, &mut Transform), Without<Player>>,
    commands: &mut Commands<'_, '_>,
) {
    let mut bullet_entities_by_id: HashMap<String, Entity> =
        bullets.iter().map(|(e, b, _)| (b.id.clone(), e)).collect();

    for bullet_state in game_state.bullets.iter() {
        bullet_entities_by_id.remove(&bullet_state.id);

        match bullets.iter_mut().find(|(_, b, _)| b.id == bullet_state.id) {
            None => {
                commands.spawn((
                    Name::new(format!("Bullet {}", bullet_state.id)),
                    Bullet {
                        id: bullet_state.id.clone(),
                    },
                    Transform::from_translation(bullet_state.position.clone().unwrap().into()),
                ));
            }
            Some((_, _, mut transform)) => {
                transform.translation = bullet_state.position.clone().unwrap().into();
                transform.rotation = bullet_state.rotation.clone().unwrap().into();
            }
        }
    }

    for (_, entity) in bullet_entities_by_id {
        commands.entity(entity).insert(Despawn);
    }
}

fn sync_players(
    game_state: &applesauce::GameState,
    mut players: Query<'_, '_, (Entity, &mut Player, &mut Transform), Without<Bullet>>,
    commands: &mut Commands<'_, '_>,
) {
    let mut player_entities_by_id: HashMap<String, Entity> =
        players.iter().map(|(e, b, _)| (b.id.clone(), e)).collect();

    for player_state in game_state.players.iter() {
        player_entities_by_id.remove(&player_state.id);

        match players.iter_mut().find(|(_, p, _)| p.id == player_state.id) {
            None => {
                commands.spawn((
                    Name::new(format!("Player {}", player_state.id)),
                    Player {
                        id: player_state.id.clone(),
                        client_id: player_state.client_id.clone(),
                        spawn_id: player_state.spawn_id.clone(),
                        radius: player_state.radius,
                        color: player_state.color.clone().unwrap().into(),
                    },
                    Transform::from_translation(player_state.position.clone().unwrap().into()),
                ));
            }
            Some((_, mut player, mut transform)) => {
                player.client_id = player_state.client_id.clone();
                player.spawn_id = player_state.spawn_id.clone();
                player.radius = player_state.radius;
                player.color = player_state.color.clone().unwrap().into();
                transform.translation = player_state.position.clone().unwrap().into();
            }
        }
    }

    for (_, entity) in player_entities_by_id {
        commands.entity(entity).insert(Despawn);
    }
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

fn despawn_things_that_need_despawning(
    mut commands: Commands,
    entities: Query<Entity, With<Despawn>>,
) {
    for entity in entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

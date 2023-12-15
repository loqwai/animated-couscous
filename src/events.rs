use bevy::prelude::*;

#[derive(Event)]
pub(crate) struct PlayerSpawnEvent {
    pub(crate) client_id: String,
}

#[derive(Event)]
pub(crate) struct PlayerMoveLeftEvent {
    pub(crate) client_id: String,
}

#[derive(Event)]
pub(crate) struct PlayerMoveRightEvent {
    pub(crate) client_id: String,
}

#[derive(Event)]
pub(crate) struct PlayerJumpEvent {
    pub(crate) client_id: String,
}

#[derive(Event)]
pub(crate) struct PlayerShootEvent {
    pub(crate) client_id: String,
    pub(crate) aim: Vec2,
}

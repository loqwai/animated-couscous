use bevy::prelude::*;

#[derive(Event)]
pub(crate) struct PlayerSpawnEvent {
    pub(crate) id: String,
    pub(crate) client_id: String,
}

#[derive(Event)]
pub(crate) struct PlayerMoveLeftEvent {
    pub(crate) id: String,
    pub(crate) client_id: String,
}

#[derive(Event)]
pub(crate) struct PlayerMoveRightEvent {
    pub(crate) id: String,
    pub(crate) client_id: String,
}

#[derive(Event)]
pub(crate) struct PlayerJumpEvent {
    pub(crate) id: String,
    pub(crate) client_id: String,
}

#[derive(Event)]
pub(crate) struct PlayerShootEvent {
    pub(crate) id: String,
    pub(crate) client_id: String,
    pub(crate) aim: Vec2,
}

#[derive(Event)]
pub(crate) struct PlayerBlockEvent {
    pub(crate) id: String,
    pub(crate) client_id: String,
}

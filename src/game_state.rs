use bevy::prelude::{Color, Event, Transform};

#[derive(Event)]
pub(crate) struct GameStateEvent {
    pub(crate) timestamp: u128,
    pub(crate) players: Vec<PlayerState>,
    pub(crate) bullets: Vec<BulletState>,
}

pub(crate) struct PlayerState {
    pub(crate) id: String,
    pub(crate) transform: Transform,
    pub(crate) color: Color,
    pub(crate) radius: f32,
}

pub(crate) struct BulletState {
    pub(crate) id: String,
    pub(crate) transform: Transform,
}

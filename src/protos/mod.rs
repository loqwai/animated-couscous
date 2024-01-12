use bevy::{prelude::default, transform::components::Transform};

use crate::events::{
    PlayerBlockEvent, PlayerJumpEvent, PlayerMoveLeftEvent, PlayerMoveRightEvent, PlayerShootEvent,
    PlayerSpawnEvent,
};

pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
}

impl From<bevy::prelude::Vec2> for generated::applesauce::Vec2 {
    fn from(v: bevy::prelude::Vec2) -> Self {
        Self {
            x: v.x,
            y: v.y,
            special_fields: Default::default(),
        }
    }
}

impl From<generated::applesauce::Vec2> for bevy::prelude::Vec2 {
    fn from(v: generated::applesauce::Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl Into<protobuf::MessageField<generated::applesauce::Vec2>> for generated::applesauce::Vec2 {
    fn into(self) -> protobuf::MessageField<generated::applesauce::Vec2> {
        protobuf::MessageField(Some(Box::new(self)))
    }
}

impl From<bevy::prelude::Vec3> for generated::applesauce::Vec3 {
    fn from(v: bevy::prelude::Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            special_fields: Default::default(),
        }
    }
}

impl From<bevy::prelude::Vec2> for generated::applesauce::Vec3 {
    fn from(v: bevy::prelude::Vec2) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: 0.,
            special_fields: Default::default(),
        }
    }
}

impl Into<bevy::prelude::Vec3> for generated::applesauce::Vec3 {
    fn into(self) -> bevy::prelude::Vec3 {
        bevy::prelude::Vec3::new(self.x, self.y, self.z)
    }
}

impl Into<bevy::prelude::Vec2> for generated::applesauce::Vec3 {
    fn into(self) -> bevy::prelude::Vec2 {
        bevy::prelude::Vec2::new(self.x, self.y)
    }
}

impl Into<protobuf::MessageField<generated::applesauce::Vec3>> for generated::applesauce::Vec3 {
    fn into(self) -> protobuf::MessageField<generated::applesauce::Vec3> {
        protobuf::MessageField(Some(Box::new(self)))
    }
}

impl From<bevy::prelude::Quat> for generated::applesauce::Quat {
    fn from(v: bevy::prelude::Quat) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w: v.w,
            special_fields: Default::default(),
        }
    }
}

impl Into<protobuf::MessageField<generated::applesauce::Quat>> for generated::applesauce::Quat {
    fn into(self) -> protobuf::MessageField<generated::applesauce::Quat> {
        protobuf::MessageField(Some(Box::new(self)))
    }
}

impl Into<bevy::prelude::Quat> for generated::applesauce::Quat {
    fn into(self) -> bevy::prelude::Quat {
        bevy::prelude::Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}

impl From<bevy::prelude::Color> for generated::applesauce::Color {
    fn from(v: bevy::prelude::Color) -> Self {
        Self {
            r: v.r(),
            g: v.g(),
            b: v.b(),
            special_fields: Default::default(),
        }
    }
}

impl From<generated::applesauce::Color> for bevy::prelude::Color {
    fn from(v: generated::applesauce::Color) -> Self {
        Self::rgb(v.r, v.g, v.b)
    }
}

impl Into<protobuf::MessageField<generated::applesauce::Color>> for generated::applesauce::Color {
    fn into(self) -> protobuf::MessageField<generated::applesauce::Color> {
        protobuf::MessageField(Some(Box::new(self)))
    }
}

impl From<&PlayerSpawnEvent> for generated::applesauce::Input {
    fn from(value: &PlayerSpawnEvent) -> Self {
        generated::applesauce::Input {
            id: value.id.to_string(),
            client_id: value.client_id.to_string(),
            inner: Some(generated::applesauce::input::Inner::Spawn(
                generated::applesauce::Spawn::default(),
            )),
            special_fields: default(),
        }
    }
}

impl From<&PlayerMoveLeftEvent> for generated::applesauce::Input {
    fn from(value: &PlayerMoveLeftEvent) -> Self {
        generated::applesauce::Input {
            id: value.id.to_string(),
            client_id: value.client_id.to_string(),
            inner: Some(generated::applesauce::input::Inner::MoveLeft(
                generated::applesauce::MoveLeft::default(),
            )),
            special_fields: default(),
        }
    }
}

impl From<&PlayerMoveRightEvent> for generated::applesauce::Input {
    fn from(value: &PlayerMoveRightEvent) -> Self {
        generated::applesauce::Input {
            id: value.id.to_string(),
            client_id: value.client_id.to_string(),
            inner: Some(generated::applesauce::input::Inner::MoveRight(
                generated::applesauce::MoveRight::default(),
            )),
            special_fields: default(),
        }
    }
}

impl From<&PlayerJumpEvent> for generated::applesauce::Input {
    fn from(value: &PlayerJumpEvent) -> Self {
        generated::applesauce::Input {
            id: value.id.to_string(),
            client_id: value.client_id.to_string(),
            inner: Some(generated::applesauce::input::Inner::Jump(
                generated::applesauce::Jump::default(),
            )),
            special_fields: default(),
        }
    }
}

impl From<&PlayerShootEvent> for generated::applesauce::Input {
    fn from(value: &PlayerShootEvent) -> Self {
        generated::applesauce::Input {
            id: value.id.to_string(),
            client_id: value.client_id.to_string(),
            inner: Some(generated::applesauce::input::Inner::Shoot(
                generated::applesauce::Shoot {
                    aim: generated::applesauce::Vec3::from(value.aim).into(),
                    special_fields: default(),
                },
            )),
            special_fields: default(),
        }
    }
}

impl From<&PlayerBlockEvent> for generated::applesauce::Input {
    fn from(value: &PlayerBlockEvent) -> Self {
        generated::applesauce::Input {
            id: value.id.to_string(),
            client_id: value.client_id.to_string(),
            inner: Some(generated::applesauce::input::Inner::Block(
                generated::applesauce::Block::default(),
            )),
            special_fields: default(),
        }
    }
}

impl From<generated::applesauce::GameState> for crate::manage_state::GameStateEvent {
    fn from(value: generated::applesauce::GameState) -> Self {
        Self {
            timestamp: value.timestamp,
            players: value
                .players
                .into_iter()
                .map(|player| crate::manage_state::PlayerState {
                    id: player.id,
                    client_id: player.client_id,
                    spawn_id: player.spawn_id,
                    radius: player.radius,
                    position: player.position.unwrap().into(),
                    color: player.color.unwrap().into(),
                    velocity: player.velocity.unwrap().into(),
                })
                .collect(),
            bullets: value
                .bullets
                .into_iter()
                .map(|bullet| crate::manage_state::BulletState {
                    id: bullet.id,
                    transform: Transform {
                        translation: bullet.position.unwrap().into(),
                        rotation: bullet.rotation.unwrap().into(),
                        ..Default::default()
                    },
                    velocity: bullet.velocity.unwrap().into(),
                })
                .collect(),
        }
    }
}

pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
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

impl Into<protobuf::MessageField<generated::applesauce::Vec3>> for generated::applesauce::Vec3 {
    fn into(self) -> protobuf::MessageField<generated::applesauce::Vec3> {
        protobuf::MessageField(Some(Box::new(self)))
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

impl Into<bevy::prelude::Color> for generated::applesauce::Color {
    fn into(self) -> bevy::prelude::Color {
        bevy::prelude::Color::rgb(self.r, self.g, self.b)
    }
}

impl Into<protobuf::MessageField<generated::applesauce::Color>> for generated::applesauce::Color {
    fn into(self) -> protobuf::MessageField<generated::applesauce::Color> {
        protobuf::MessageField(Some(Box::new(self)))
    }
}

impl From<(bool, bool)> for generated::applesauce::MoveData {
    fn from((moving_left, moving_right): (bool, bool)) -> Self {
        Self {
            moving_left,
            moving_right,
            special_fields: Default::default(),
        }
    }
}

impl Into<protobuf::MessageField<generated::applesauce::MoveData>>
    for generated::applesauce::MoveData
{
    fn into(self) -> protobuf::MessageField<generated::applesauce::MoveData> {
        protobuf::MessageField(Some(Box::new(self)))
    }
}

impl From<String> for generated::applesauce::DespawnPlayer {
    fn from(player_id: String) -> Self {
        Self {
            player_id,
            special_fields: Default::default(),
        }
    }
}

impl Into<generated::applesauce::Wrapper> for generated::applesauce::DespawnPlayer {
    fn into(self) -> generated::applesauce::Wrapper {
        generated::applesauce::wrapper::Inner::DespawnPlayer(self).into()
    }
}

impl Into<generated::applesauce::Wrapper> for generated::applesauce::Bullet {
    fn into(self) -> generated::applesauce::Wrapper {
        generated::applesauce::wrapper::Inner::Bullet(self).into()
    }
}

impl Into<generated::applesauce::Wrapper> for generated::applesauce::Block {
    fn into(self) -> generated::applesauce::Wrapper {
        generated::applesauce::wrapper::Inner::Block(self).into()
    }
}

impl Into<generated::applesauce::Wrapper> for generated::applesauce::Player {
    fn into(self) -> generated::applesauce::Wrapper {
        generated::applesauce::wrapper::Inner::Player(self).into()
    }
}

impl Into<generated::applesauce::Wrapper> for generated::applesauce::OutOfSync {
    fn into(self) -> generated::applesauce::Wrapper {
        generated::applesauce::wrapper::Inner::OutOfSync(self).into()
    }
}

impl Into<generated::applesauce::Wrapper> for generated::applesauce::Jump {
    fn into(self) -> generated::applesauce::Wrapper {
        generated::applesauce::wrapper::Inner::Jump(self).into()
    }
}

impl From<generated::applesauce::wrapper::Inner> for generated::applesauce::Wrapper {
    fn from(value: generated::applesauce::wrapper::Inner) -> Self {
        generated::applesauce::Wrapper {
            id: uuid::Uuid::new_v4().to_string(),
            inner: Some(value),
            special_fields: Default::default(),
        }
    }
}

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

// impl From<(bool, bool)> for generated::applesauce::MoveData {
//     fn from((moving_left, moving_right): (bool, bool)) -> Self {
//         Self {
//             moving_left,
//             moving_right,
//             special_fields: Default::default(),
//         }
//     }
// }

// impl Into<protobuf::MessageField<generated::applesauce::MoveData>>
//     for generated::applesauce::MoveData
// {
//     fn into(self) -> protobuf::MessageField<generated::applesauce::MoveData> {
//         protobuf::MessageField(Some(Box::new(self)))
//     }
// }

// impl From<String> for generated::applesauce::DespawnPlayer {
//     fn from(player_id: String) -> Self {
//         Self {
//             player_id,
//             special_fields: Default::default(),
//         }
//     }
// }

// impl From<generated::applesauce::DespawnPlayer> for generated::applesauce::Wrapper {
//     fn from(value: generated::applesauce::DespawnPlayer) -> Self {
//         generated::applesauce::wrapper::Inner::DespawnPlayer(value).into()
//     }
// }

// impl From<generated::applesauce::Bullet> for generated::applesauce::Wrapper {
//     fn from(value: generated::applesauce::Bullet) -> Self {
//         generated::applesauce::wrapper::Inner::Bullet(value).into()
//     }
// }

// impl From<generated::applesauce::Block> for generated::applesauce::Wrapper {
//     fn from(value: generated::applesauce::Block) -> Self {
//         generated::applesauce::wrapper::Inner::Block(value).into()
//     }
// }

// impl From<generated::applesauce::Player> for generated::applesauce::Wrapper {
//     fn from(value: generated::applesauce::Player) -> Self {
//         generated::applesauce::wrapper::Inner::Player(value).into()
//     }
// }

// impl From<generated::applesauce::OutOfSync> for generated::applesauce::Wrapper {
//     fn from(value: generated::applesauce::OutOfSync) -> Self {
//         generated::applesauce::wrapper::Inner::OutOfSync(value).into()
//     }
// }

// impl From<generated::applesauce::Jump> for generated::applesauce::Wrapper {
//     fn from(value: generated::applesauce::Jump) -> Self {
//         generated::applesauce::wrapper::Inner::Jump(value).into()
//     }
// }

// impl From<generated::applesauce::State> for generated::applesauce::Wrapper {
//     fn from(value: generated::applesauce::State) -> Self {
//         generated::applesauce::wrapper::Inner::State(value).into()
//     }
// }

// impl From<generated::applesauce::wrapper::Inner> for generated::applesauce::Wrapper {
//     fn from(value: generated::applesauce::wrapper::Inner) -> Self {
//         generated::applesauce::Wrapper {
//             id: uuid::Uuid::new_v4().to_string(),
//             inner: Some(value),
//             special_fields: Default::default(),
//         }
//     }
// }

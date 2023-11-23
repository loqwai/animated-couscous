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

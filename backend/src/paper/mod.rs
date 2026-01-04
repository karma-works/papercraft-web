mod craft;
mod model;
mod types;

pub use craft::*;
pub use model::*;
pub use types::*;

use crate::util_3d::*;
use serde::{
    Deserialize,
    ser::{SerializeSeq, SerializeStruct},
};
use tr::tr;

pub fn signature() -> String {
    tr!("Created with Papercraft Web. https://github.com/karma-works/papercraft-web")
}

mod ser {
    use super::*;
    pub mod vector2 {
        use super::*;
        pub fn serialize<S>(data: &Vector2, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(3))?;
            seq.serialize_element(&data.x)?;
            seq.serialize_element(&data.y)?;
            seq.end()
        }
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vector2, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let data = <[f32; 2]>::deserialize(deserializer)?;
            Ok(Vector2::from(data))
        }
    }
    pub mod vector3 {
        use super::*;
        pub fn serialize<S>(data: &Vector3, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(3))?;
            seq.serialize_element(&data.x)?;
            seq.serialize_element(&data.y)?;
            seq.serialize_element(&data.z)?;
            seq.end()
        }
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vector3, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let data = <[f32; 3]>::deserialize(deserializer)?;
            Ok(Vector3::from(data))
        }
    }
}

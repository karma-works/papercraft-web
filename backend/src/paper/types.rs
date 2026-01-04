use serde::{Deserialize, Serialize};
use crate::util_3d::Vector2;
use cgmath::Rad;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Near,
    Center,
    Far,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrintableText {
    pub text: String,
    pub pos: Vector2,
    pub angle: Rad<f32>,
    pub size: f32, //mm
    pub align: TextAlign,
}

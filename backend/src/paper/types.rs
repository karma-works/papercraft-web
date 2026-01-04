use serde::{Deserialize, Serialize};
use crate::util_3d::Vector2;
use cgmath::Rad;
use crate::paper::{IslandKey, PaperOptions};
use slotmap::Key;

// Re-export ModelInfo since it's used in RenderablePapercraft but defined in model.rs
// Note: We might need to handle circular deps if moved improperly, but types.rs is a leaf.
// Actually ModelInfo is in model.rs. We need to be careful with imports.
// Let's just use generic or specific types.

#[derive(Serialize)]
pub struct RenderablePapercraft {
    pub model: crate::paper::Model,
    pub islands: Vec<RenderableIsland>,
    pub options: PaperOptions,
}

#[derive(Serialize)]
pub struct RenderableIsland {
    pub id: IslandKey,
    pub pos: Vector2,
    pub rot: f32,
    pub faces: Vec<RenderableFace>,
    pub edges: Vec<RenderableEdge>,
    pub flaps: Vec<RenderableFlap>,
}

#[derive(Serialize)]
pub struct RenderableFace {
    pub id: crate::paper::FaceIndex,
    pub vertices: Vec<Vector2>,
}

#[derive(Serialize)]
pub struct RenderableEdge {
    pub id: crate::paper::EdgeIndex,
    pub start: Vector2,
    pub end: Vector2,
    pub kind: String, // "cut", "mountain", "valley"
}

#[derive(Serialize)]
pub struct RenderableFlap {
    pub id: crate::paper::FaceIndex, // Associated face
    pub vertices: Vec<Vector2>,
}

impl RenderableIsland {
    // Helper to keep struct definition clean above
}

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

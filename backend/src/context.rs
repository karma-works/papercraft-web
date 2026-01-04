use std::path::PathBuf;
use crate::paper::{Papercraft, PaperOptions};
use crate::config;

pub struct GlobalContext {
    pub config: config::Config,
    pub data: PapercraftState,
    pub last_export: PathBuf,
    pub last_export_filter: Option<i32>,
    pub font_text_line_scale: f32,
}

pub struct PapercraftState {
    pub project: Papercraft,
}

impl PapercraftState {
    pub fn papercraft(&self) -> &Papercraft {
        &self.project
    }
}

impl GlobalContext {
    pub fn title(&self, _modified: bool) -> String {
        "Papercraft Web".to_string()
    }
}

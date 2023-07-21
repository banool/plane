use bevy::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Resource)]
pub struct Args {
    pub matchbox: String,

    pub room: Option<String>,

    pub players: usize,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            matchbox: "ws://127.0.0.1:3536".to_string(),
            room: None,
            players: 2,
        }
    }
}

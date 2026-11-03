use super::Recovery;
use crate::{
    fixture::{
        DeltaFixture, Release,
        recovery::{FILE_COUNT, archive, filename, transfer_payload},
    },
    server::WgUpdateService,
    support::{Sandbox, success},
};
use std::{fs, path::Path};

fn size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

mod delta;
mod downloads;
mod extraction;
mod finalization;

fn assert_state(game: &Path, client: &str, locale: &str) {
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(game.join(".wg-launcher/state.json")).unwrap()).unwrap();
    assert_eq!(state["version"], client);
    assert_eq!(
        state["component_versions"],
        serde_json::json!({"client": client, "locale": locale})
    );
    assert_eq!(state["product"]["game_id"], "LOCAL.WOT");
    assert_eq!(state["region"], "local");
}

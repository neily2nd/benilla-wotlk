//! Entry point for the isolated offline player-control proof of concept.

use bevy::prelude::AppExit;

/// Start Benilla's world viewer and install only the local player controller adapter.
pub fn run() -> AppExit {
    benilla_world::worldview::run_with(crate::BuildId::default(), |app| {
        app.insert_resource(benilla_world::worldview::WorldviewFlyCam(false))
            .add_plugins(crate::player::offline::OfflinePlayerPlugin);
    })
}

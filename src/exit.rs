use std::sync::atomic::{AtomicBool, Ordering};
use valence::prelude::*;

static EXIT: AtomicBool = AtomicBool::new(false);

pub struct Exit;

fn exit() {
    println!();
    EXIT.store(true, Ordering::Relaxed);
}

pub fn handle_exit(mut events: EventWriter<AppExit>) {
    if EXIT.load(Ordering::Relaxed) {
        events.send(AppExit::from_code(0));
    }
}

impl Plugin for Exit {
    fn build(&self, app: &mut App) {
        let result = ctrlc::try_set_handler(exit);
        if let Err(err) = result {
            tracing::warn!(%err, "Failed to set `Ctrl+C` handler");
        }

        app.add_systems(PreUpdate, handle_exit);
    }
}

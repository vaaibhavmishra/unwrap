mod context;
pub mod context_provider;
mod events;
mod macros;
pub mod rudder_message;
pub mod secret_redaction;

pub use context::telemetry_context;
pub use events::*;

/// Removes all events from the in-memory telemetry queue.
pub fn clear_event_queue() {
    let _ = warpui::telemetry::flush_events();
}

use tracing::subscriber;

pub fn init() -> anyhow::Result<Initialization> {
    subscriber::set_global_default(subscriber::NoSubscriber::new())?;
    Ok(Initialization)
}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn init_child_process() -> anyhow::Result<(Initialization, opentelemetry::Context)> {
    Ok((init()?, opentelemetry::Context::new()))
}

pub struct Initialization;

impl Initialization {
    pub fn log_initialization_warning(&mut self) {}

    pub(crate) fn shutdown(&mut self) {}
}

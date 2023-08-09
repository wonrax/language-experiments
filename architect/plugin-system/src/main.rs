use std::collections::HashMap;

use log::{error, info};
use pretty_env_logger;
mod plugin_core;
use plugin_core::*;
mod tests;

#[derive(Debug)]
struct APlugin {}
impl IPlugin for APlugin {}

fn main() {
    pretty_env_logger::init();

    let mut pm = PluginManager {
        plugin_states: HashMap::new(),
        plugins: HashMap::new(),
    };

    match pm.register_plugin(
        APlugin {},
        &PluginConfig {
            environment: "prod".into(),
        },
    ) {
        Ok(p) => info!("Registered {}", p.get_name()),
        Err(err) => error!("Error: {}", err),
    }

    match pm.start_plugin("plugin_system::APlugin".into()) {
        Ok(()) => info!("Started plugin {}", "plugin_system::APlugin"),
        Err(err) => error!("Couldn't start plugin: {}", err),
    }
}

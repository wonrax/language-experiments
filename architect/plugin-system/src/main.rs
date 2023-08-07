use std::collections::HashMap;

enum PluginState {
    Enabled,
    Started,
    Disabled, // or stopped
    Destroyed
}

struct PluginManager {
    plugin_states: HashMap<String, PluginState>,
    plugins: Vec<Box<dyn IPlugin>>
}

trait IPluginManager {
    fn register_plugin<'a>(&mut self, p: impl IPlugin + 'static) -> Result<&Box<dyn IPlugin>, String>;
}

impl IPluginManager for PluginManager {
    fn register_plugin(&mut self, p: impl IPlugin + 'static) -> Result<&Box<dyn IPlugin>, String> {
        let type_name = p.get_name();
        if self.plugin_states.contains_key(type_name) {
            return Err("Plugin already exists".into());
        }

        self.plugin_states.insert(type_name.into(), PluginState::Enabled);

        let _p = Box::new(p);
        self.plugins.push(_p);

        Ok(self.plugins.last().unwrap())
    }
}

struct APlugin {}

impl IPlugin for APlugin {}

trait IPlugin {
    fn on_enabled(&self) {}
    fn on_start(&self) {}
    fn on_disabled(&self) {}
    fn on_destroy(&self) {}
    fn get_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

fn main() {
    let mut pm = PluginManager {
        plugin_states: HashMap::new(),
        plugins: Vec::new()
    };

    match pm.register_plugin(APlugin {}) {
        Ok(p) => println!("Registered {}", p.get_name()),
        Err(err) => println!("Error: {}", err),
    }
}

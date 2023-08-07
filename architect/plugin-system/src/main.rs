use std::{
    collections::HashMap, error::{self, Error},
};

#[derive(Debug)]
enum PluginState {
    Enabled,
    Started,
    Disabled, // or stopped
    Destroyed,
}

struct PluginManager {
    plugin_states: HashMap<String, PluginState>,
    plugins: HashMap<String, Box<dyn IPlugin>>,
}

impl PluginManager {
    fn register_plugin(
        &mut self,
        mut p: impl IPlugin + 'static,
    ) -> Result<&Box<dyn IPlugin>, String> {
        let type_name = p.get_name().to_string();

        if self.plugin_states.contains_key(&type_name) {
            return Err("Plugin already exists".into());
        }

        p.on_enabled();

        let _p = Box::new(p);
        self.plugins.insert(type_name.clone(), _p);

        self.plugin_states
            .insert(type_name.clone(), PluginState::Enabled);

        match self.plugins.get(&type_name) {
            Some(plugin_ref) => Ok(plugin_ref),
            None => Err("smth went wrong bruh".into()),
        }
    }

    fn start_plugin(&mut self, plugin_name: String) -> Result<(), Box<dyn Error>> {
        match self.plugin_states.get(&plugin_name) {
            Some(state) => match state {
                PluginState::Enabled => match self.plugins.get_mut(&plugin_name) {
                    Some(p) => {
                        p.on_start();
                        self.plugin_states.insert(plugin_name, PluginState::Started);
                        Ok(())
                    }
                    _ => Error::new,
                },
                _ => ()
            },
            _ => (),
        }
    }

    fn disable_plugin(&mut self, plugin_name: String) {
        match self.plugins.get_mut(&plugin_name) {
            Some(p) => {
                p.on_disabled();
                self.plugin_states
                    .insert(plugin_name, PluginState::Disabled);
            }
            _ => (),
        }
    }

    fn destroy_plugin(&mut self, plugin_name: String) {
        match self.plugins.get_mut(&plugin_name) {
            Some(p) => {
                p.on_destroy();
                self.plugin_states
                    .insert(plugin_name, PluginState::Destroyed);
            }
            _ => (),
        }
    }
}

struct APlugin {}
impl IPlugin for APlugin {}

trait IPlugin {
    fn on_enabled(&mut self) {}
    fn on_start(&mut self) {}
    fn on_disabled(&mut self) {}
    fn on_destroy(&mut self) {}
    fn get_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

fn main() {
    let mut pm = PluginManager {
        plugin_states: HashMap::new(),
        plugins: HashMap::new(),
    };

    match pm.register_plugin(APlugin {}) {
        Ok(p) => println!("Registered {}", p.get_name()),
        Err(err) => println!("Error: {}", err),
    }
    match pm.register_plugin(APlugin {}) {
        Ok(p) => println!("Registered {}", p.get_name()),
        Err(err) => println!("Error: {}", err),
    }

    // println!("{:?}", pm.plugin_states.get(&Box::new(APlugin{})));
}

use std::{collections::HashMap, fmt::{Display, Debug}};
use thiserror;

pub trait IPlugin: Debug {
    fn on_enabled(&mut self) {}
    fn on_start(&mut self) {}
    fn on_disabled(&mut self) {}
    fn on_destroy(&mut self) {}
    fn get_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

#[derive(Debug, Clone)]
pub enum PluginState {
    Enabled,
    Started,
    Disabled, // or stopped
    Destroyed,
}

pub struct PluginManager {
    pub plugin_states: HashMap<String, PluginState>,
    pub plugins: HashMap<String, Box<dyn IPlugin>>,
}

#[derive(thiserror::Error, Debug)]
pub enum PluginSystemError {
    #[error("Plugin `{0}` does not exist")]
    PluginNotExists(String),
    #[error("Plugin `{0}` already exists")]
    PluginAlreadyExists(String),
    #[error("Invalid state transition, current state: `{0}`, action: `{1}`")]
    InvalidStateTransition(PluginState, String),
    #[error("{0}")]
    Unknown(String),
}

impl PluginManager {
    pub fn register_plugin(
        &mut self,
        mut p: impl IPlugin + 'static,
    ) -> Result<&Box<dyn IPlugin>, PluginSystemError> {
        let type_name = p.get_name().to_string();

        if self.plugin_states.contains_key(&type_name) {
            return Err(PluginSystemError::PluginAlreadyExists(type_name));
        }

        p.on_enabled();

        let _p = Box::new(p);
        self.plugins.insert(type_name.clone(), _p);

        self.plugin_states
            .insert(type_name.clone(), PluginState::Enabled);

        match self.plugins.get(&type_name) {
            Some(plugin_ref) => Ok(plugin_ref),
            None => Err(PluginSystemError::Unknown(
                "Plugin registered but couldn't get the reference to it".into(),
            )),
        }
    }

    pub fn start_plugin(&mut self, plugin_name: String) -> Result<(), PluginSystemError> {
        match self.plugin_states.get(&plugin_name) {
            Some(state) => match state {
                PluginState::Enabled => match self.plugins.get_mut(&plugin_name) {
                    Some(p) => {
                        p.on_start();
                        self.plugin_states.insert(plugin_name, PluginState::Started);
                        Ok(())
                    }
                    _ => Err(PluginSystemError::PluginNotExists(plugin_name)),
                },
                rest => Err(PluginSystemError::InvalidStateTransition(
                    rest.clone(),
                    "start_plugin".into(),
                )),
            },
            _ => Err(PluginSystemError::PluginNotExists(plugin_name)),
        }
    }

    fn disable_plugin(&mut self, plugin_name: String) -> Result<(), PluginSystemError> {
        match self.plugin_states.get(&plugin_name) {
            Some(state) => match state {
                PluginState::Enabled | PluginState::Started => {
                    match self.plugins.get_mut(&plugin_name) {
                        Some(p) => {
                            p.on_disabled();
                            self.plugin_states.insert(plugin_name, PluginState::Started);
                            Ok(())
                        }
                        _ => Err(PluginSystemError::PluginNotExists(plugin_name)),
                    }
                }
                rest => Err(PluginSystemError::InvalidStateTransition(
                    rest.clone(),
                    "start_plugin".into(),
                )),
            },
            _ => Err(PluginSystemError::PluginNotExists(plugin_name)),
        }
    }

    fn destroy_plugin(&mut self, plugin_name: String) -> Result<(), PluginSystemError> {
        match self.plugin_states.get(&plugin_name) {
            Some(state) => match state {
                PluginState::Enabled | PluginState::Disabled => {
                    match self.plugins.get_mut(&plugin_name) {
                        Some(p) => {
                            p.on_destroy();
                            self.plugin_states
                                .insert(plugin_name, PluginState::Destroyed);
                            Ok(())
                        }
                        _ => Err(PluginSystemError::PluginNotExists(plugin_name)),
                    }
                }
                PluginState::Started => match self.plugins.get_mut(&plugin_name) {
                    Some(p) => {
                        p.on_disabled();
                        self.plugin_states
                            .insert(plugin_name.clone(), PluginState::Disabled);

                        p.on_destroy();
                        self.plugin_states
                            .insert(plugin_name, PluginState::Destroyed);

                        Ok(())
                    }
                    _ => Err(PluginSystemError::PluginNotExists(plugin_name)),
                },
                rest => Err(PluginSystemError::InvalidStateTransition(
                    rest.clone(),
                    "start_plugin".into(),
                )),
            },
            _ => Err(PluginSystemError::PluginNotExists(plugin_name)),
        }
    }
}

impl Display for PluginState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<Self>();
        write!(f, "{}", name)
    }
}

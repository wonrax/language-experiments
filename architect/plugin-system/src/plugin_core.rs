use std::{
    any::Any,
    collections::HashMap,
    fmt::{Debug, Display},
};
use thiserror;

pub trait IPlugin: Debug + AToAny {
    fn on_enabled(&mut self, _: &PluginConfig) {}
    fn on_start(&mut self) {}
    fn on_disabled(&mut self) {}
    fn on_destroy(&mut self) {}
    fn get_name<'a>(&self) -> &'a str {
        std::any::type_name::<Self>()
    }
}

pub struct PluginConfig {
    pub environment: String,
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

    #[error("Couldn't cast to plugin this type")]
    InvalidPluginType,

    #[error("{0}")]
    Unknown(String),
}

pub trait AToAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> AToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl PluginManager {
    pub fn register_plugin<T: IPlugin + 'static>(
        &mut self,
        mut p: T,
        plugin_config: &PluginConfig,
    ) -> Result<&T, PluginSystemError> {
        let type_name = p.get_name().to_string();

        if self.plugin_states.contains_key(&type_name) {
            return Err(PluginSystemError::PluginAlreadyExists(type_name));
        }

        p.on_enabled(plugin_config);

        let _p = Box::new(p);
        self.plugins.insert(type_name.clone(), _p);

        self.plugin_states
            .insert(type_name.clone(), PluginState::Enabled);

        match self.plugins.get(&type_name) {
            // I don't even know how this works, someone pls explain
            Some(p) => Ok(p.as_ref().as_any().downcast_ref::<T>().unwrap()),
            None => Err(PluginSystemError::Unknown(
                "Plugin registered but couldn't get the reference to it".into(),
            )),
        }
    }

    pub fn start_plugin(&mut self, plugin_name: &str) -> Result<(), PluginSystemError> {
        match self.plugin_states.get(plugin_name) {
            Some(state) => match state {
                PluginState::Enabled => match self.plugins.get_mut(plugin_name) {
                    Some(p) => {
                        p.on_start();
                        self.plugin_states
                            .insert(plugin_name.to_string(), PluginState::Started);
                        Ok(())
                    }
                    _ => Err(PluginSystemError::PluginNotExists(plugin_name.to_string())),
                },
                rest => Err(PluginSystemError::InvalidStateTransition(
                    rest.clone(),
                    "start_plugin".into(),
                )),
            },
            _ => Err(PluginSystemError::PluginNotExists(plugin_name.to_string())),
        }
    }

    pub fn disable_plugin(&mut self, plugin_name: &str) -> Result<(), PluginSystemError> {
        match self.plugin_states.get(plugin_name) {
            Some(state) => match state {
                PluginState::Enabled | PluginState::Started => {
                    match self.plugins.get_mut(plugin_name) {
                        Some(p) => {
                            p.on_disabled();
                            self.plugin_states
                                .insert(plugin_name.to_string(), PluginState::Started);
                            Ok(())
                        }
                        _ => Err(PluginSystemError::PluginNotExists(plugin_name.to_string())),
                    }
                }
                rest => Err(PluginSystemError::InvalidStateTransition(
                    rest.clone(),
                    "start_plugin".into(),
                )),
            },
            _ => Err(PluginSystemError::PluginNotExists(plugin_name.to_string())),
        }
    }

    pub fn destroy_plugin(&mut self, plugin_name: &str) -> Result<(), PluginSystemError> {
        match self.plugin_states.get(plugin_name) {
            Some(state) => match state {
                PluginState::Enabled | PluginState::Disabled => {
                    match self.plugins.get_mut(plugin_name) {
                        Some(p) => {
                            p.on_destroy();
                            self.plugin_states
                                .insert(plugin_name.to_string(), PluginState::Destroyed);
                            Ok(())
                        }
                        _ => Err(PluginSystemError::PluginNotExists(plugin_name.to_string())),
                    }
                }
                PluginState::Started => match self.plugins.get_mut(plugin_name) {
                    Some(p) => {
                        p.on_disabled();
                        self.plugin_states
                            .insert(plugin_name.to_string(), PluginState::Disabled);

                        p.on_destroy();
                        self.plugin_states
                            .insert(plugin_name.to_string(), PluginState::Destroyed);

                        Ok(())
                    }
                    _ => Err(PluginSystemError::PluginNotExists(plugin_name.to_string())),
                },
                rest => Err(PluginSystemError::InvalidStateTransition(
                    rest.clone(),
                    "start_plugin".into(),
                )),
            },
            _ => Err(PluginSystemError::PluginNotExists(plugin_name.to_string())),
        }
    }

    pub fn get_plugin_ref<T: 'static>(&self, name: &str) -> Result<&T, PluginSystemError> {
        match self.plugins.get(name) {
            // I don't even know how this works, someone pls explain
            Some(p) => match p.as_ref().as_any().downcast_ref::<T>() {
                Some(p) => Ok(p),
                None => Err(PluginSystemError::InvalidPluginType),
            },
            None => Err(PluginSystemError::PluginNotExists(name.to_string())),
        }
    }
}

impl Display for PluginState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<Self>();
        write!(f, "{}", name)
    }
}

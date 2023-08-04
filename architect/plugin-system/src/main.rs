use std::{collections::HashMap, hash::Hash};

struct PluginManager<'a> {
    plugins: Vec<Box<dyn IPlugin>>,
    plugin_state: HashMap<Box<&'a dyn IPlugin>, bool>,
}

impl<'a> PartialEq for Box<&'a dyn IPlugin> {
    fn eq(&self, other: &Self) -> bool {
        return true;
    }
}

impl<'a> Eq for Box<&'a dyn IPlugin> {

}

impl<'a> Hash for Box<&'a dyn IPlugin> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {}
}

trait IPluginManager {
    fn register_plugin(&mut self, p: &impl IPlugin);
}

fn _get_name<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

impl IPluginManager for PluginManager<'_> {
    fn register_plugin<'a>(&mut self, p: &'a impl IPlugin) {
        let type_name = _get_name(&p);
        println!("{type_name}");
        self.plugin_state.insert(Box::new(p as &'a dyn IPlugin), true).expect("must");
    }
}

struct APlugin {}

impl IPlugin for APlugin {}

trait IPlugin {
    fn on_enabled(&self) {}
    fn on_start(&self) {}
    fn on_disabled(&self) {}
    fn on_destroy(&self) {}
}

fn main() {
    let mut pm = PluginManager {
        plugins: vec![],
        plugin_state: HashMap::new(),
    };
    pm.register_plugin(&APlugin {})
}

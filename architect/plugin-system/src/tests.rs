#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::plugin_core::*;

    #[derive(Debug)]
    struct APlugin {}
    impl IPlugin for APlugin {}

    #[test]
    fn test_s() {
        let mut pm = PluginManager {
            plugin_states: HashMap::new(),
            plugins: HashMap::new(),
        };

        pm.register_plugin(APlugin {}).expect("should not fail");
        pm.register_plugin(APlugin {})
            .expect_err("should fail because of duplicate plugin");
    }
}

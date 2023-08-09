#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::plugin_core::*;

    #[derive(Debug)]
    struct TestPlugin {}
    impl IPlugin for TestPlugin {}

    #[test]
    fn test_plugin_registration() {
        let mut pm = PluginManager {
            plugin_states: HashMap::new(),
            plugins: HashMap::new(),
        };

        let plugin_config = PluginConfig {
            environment: "test".into(),
        };

        pm.register_plugin(TestPlugin {}, &plugin_config)
            .expect("should not fail");
        pm.register_plugin(TestPlugin {}, &plugin_config)
            .expect_err("should fail because of duplicate plugin");
    }

    #[derive(Debug)]
    struct TestLifecyclePlugin {
        env: String,
        i: i32,
    }

    impl IPlugin for TestLifecyclePlugin {
        fn on_enabled(&mut self, plugin_config: &PluginConfig) {
            self.i = 1;
            self.env = plugin_config.environment.clone();
        }

        fn on_start(&mut self) {
            self.i = 2;
        }

        fn on_disabled(&mut self) {
            self.i = 0;
        }

        fn on_destroy(&mut self) {
            self.i = -1;
        }
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut pm = PluginManager {
            plugin_states: HashMap::new(),
            plugins: HashMap::new(),
        };

        let plugin_config = PluginConfig {
            environment: "test".into(),
        };

        let plugin = pm
            .register_plugin(
                TestLifecyclePlugin {
                    env: "".into(),
                    i: 0,
                },
                &plugin_config,
            )
            .expect("should not fail");

        let plugin_name: String = plugin.get_name().into();

        assert_eq!(plugin.env, "test");
        assert_eq!(plugin.i, 1);

        {
            pm.start_plugin(plugin_name.clone()).unwrap();

            let plugin = pm
                .get_plugin_ref::<TestLifecyclePlugin>(&plugin_name)
                .unwrap();

            assert_eq!(plugin.i, 2);
        }

        {
            pm.disable_plugin(plugin_name.clone()).unwrap();

            let plugin = pm
                .get_plugin_ref::<TestLifecyclePlugin>(&plugin_name)
                .unwrap();

            assert_eq!(plugin.i, 0);
        }

        {
            pm.destroy_plugin(plugin_name.clone()).unwrap();

            let plugin = pm
                .get_plugin_ref::<TestLifecyclePlugin>(&plugin_name)
                .unwrap();

            assert_eq!(plugin.i, -1);
        }
    }
}

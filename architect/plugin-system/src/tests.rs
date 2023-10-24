#[cfg(test)]
mod test {
    use std::{collections::HashMap, marker::PhantomData};

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
        let mut plugin_manager = PluginManager {
            plugin_states: HashMap::new(),
            plugins: HashMap::new(),
        };

        let plugin_config = PluginConfig {
            environment: "test".into(),
        };

        let plugin = plugin_manager
            .register_plugin(
                TestLifecyclePlugin {
                    env: "".into(),
                    i: 0,
                },
                &plugin_config,
            )
            .expect("should not fail");

        let plugin_name = plugin.get_name();

        {
            let plugin = plugin_manager
                .get_plugin_ref::<TestLifecyclePlugin>(plugin_name)
                .unwrap();

            assert_eq!(plugin.env, "test");
            assert_eq!(plugin.i, 1);
        }

        {
            plugin_manager.start_plugin(plugin_name).unwrap();

            let plugin = plugin_manager
                .get_plugin_ref::<TestLifecyclePlugin>(plugin_name)
                .unwrap();

            assert_eq!(plugin.i, 2);
        }

        {
            plugin_manager.disable_plugin(plugin_name).unwrap();

            let plugin = plugin_manager
                .get_plugin_ref::<TestLifecyclePlugin>(plugin_name)
                .unwrap();

            assert_eq!(plugin.i, 0);
        }

        {
            plugin_manager.destroy_plugin(plugin_name).unwrap();

            let plugin = plugin_manager
                .get_plugin_ref::<TestLifecyclePlugin>(plugin_name)
                .unwrap();

            assert_eq!(plugin.i, -1);
        }
    }

    // Another way to test using helper to shorten the method call
    #[test]
    fn test_plugin_lifecycle_alternative() {
        struct TestHelper<T> {
            pm: PluginManager,
            plugin_name: String,
            phantom: PhantomData<T>,
        }

        impl<T: 'static> TestHelper<T> {
            fn get_plugin_ref(&self) -> &T {
                return self.pm.get_plugin_ref(&self.plugin_name).unwrap();
            }
        }

        let plugin = TestLifecyclePlugin {
            env: "".into(),
            i: 0,
        };

        let mut test_helper = TestHelper::<TestLifecyclePlugin> {
            pm: PluginManager {
                plugin_states: HashMap::new(),
                plugins: HashMap::new(),
            },
            plugin_name: plugin.get_name().into(),
            phantom: PhantomData,
        };

        let plugin = test_helper
            .pm
            .register_plugin(
                plugin,
                &PluginConfig {
                    environment: "test".into(),
                },
            )
            .expect("should not fail");

        let plugin_name = plugin.get_name().to_string();

        assert_eq!(plugin.env, "test");
        assert_eq!(plugin.i, 1);

        test_helper.pm.start_plugin(&plugin_name).unwrap();
        assert_eq!(test_helper.get_plugin_ref().i, 2);

        test_helper.pm.disable_plugin(&plugin_name).unwrap();
        assert_eq!(test_helper.get_plugin_ref().i, 0);

        test_helper.pm.destroy_plugin(&plugin_name).unwrap();
        assert_eq!(test_helper.get_plugin_ref().i, -1);
    }
}

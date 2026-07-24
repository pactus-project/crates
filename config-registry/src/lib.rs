use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

type PrependEnvsFn = fn();

/// Thread-safe registry using Mutex
pub struct ConfigRegistry {
    configs: HashMap<TypeId, PrependEnvsFn>,
}

impl ConfigRegistry {
    fn new() -> Self {
        ConfigRegistry {
            configs: HashMap::new(),
        }
    }

    /// Register a config type
    pub fn register<T: 'static>(&mut self, prepend_envs: PrependEnvsFn) {
        self.configs.insert(TypeId::of::<T>(), prepend_envs);
    }

    /// Apply all registered prefixes
    pub fn apply_all_prefixes(&self) {
        for prepend_fn in self.configs.values() {
            prepend_fn();
        }
    }
}

/// Global registry instance using thread-safe OnceLock<Mutex<>>
static REGISTRY: OnceLock<Mutex<ConfigRegistry>> = OnceLock::new();

/// Function to access the registry
pub fn global_registry() -> &'static Mutex<ConfigRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(ConfigRegistry::new()))
}

/// Trait for configs that can register themselves
pub trait RegisterableConfig: 'static {
    fn register_self();
}

/// Concrete type for inventory collection
pub struct ConfigRegistrationItem {
    pub register_fn: fn(),
}

// Re-export for downstream users of the macro
pub use inventory;

// Collect all registered configs
inventory::collect!(ConfigRegistrationItem);

/// Initialize all configs
pub fn init_all_configs() {
    for item in inventory::iter::<ConfigRegistrationItem> {
        (item.register_fn)();
    }
}

/// Register a configuration type with the global config registry.
///
/// # Usage
///
/// ```ignore
/// use clap::Args;
/// use config_registry::register_config;
/// use procedural_env::EnvPrefix;
///
/// #[derive(Debug, Clone, Args, EnvPrefix)]
/// #[env_prefix = "MY_APP"]
/// pub struct Config {
///     #[arg(long, env = "SERVER_PORT")]
///     pub port: u16,
/// }
///
/// register_config!(Config);
/// ```
#[macro_export]
macro_rules! register_config {
    ($config_type:ty) => {
        const _: () = {
            fn register_config_fn() {
                <$config_type as $crate::RegisterableConfig>::register_self();
            }

            impl $crate::RegisterableConfig for $config_type {
                fn register_self() {
                    let mut registry = $crate::global_registry().lock().unwrap();
                    registry.register::<Self>(Self::prepend_envs);
                }
            }

            $crate::inventory::submit! {
                $crate::ConfigRegistrationItem {
                    register_fn: register_config_fn
                }
            }
        };
    };
}

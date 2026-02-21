pub mod installer;
pub mod lifecycle;
pub mod manifest;
pub mod registry;
pub mod slots;
pub mod loader;
pub mod sdk;
pub mod permissions;
pub mod event_bus;

pub use installer::PluginInstaller;
pub use lifecycle::{DefaultPluginLifecycle, PluginLifecycle, PluginLifecycleContext, PluginState, run_load_sequence, run_unload_sequence};
pub use manifest::{PluginHookEntry, PluginManifest, PluginPermissions, PluginToolSlot};
pub use registry::PluginRegistry;
pub use slots::{collect_plugin_tools, ResolvedPluginTool};

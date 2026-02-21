pub mod installer;
pub mod lifecycle;
pub mod manifest;
pub mod registry;
pub mod slots;

pub use installer::PluginInstaller;
pub use lifecycle::{DefaultPluginLifecycle, PluginLifecycle, PluginLifecycleContext, PluginState, run_load_sequence, run_unload_sequence};
pub use manifest::{PluginHookEntry, PluginManifest, PluginPermissions, PluginToolSlot};
pub use registry::PluginRegistry;
pub use slots::{collect_plugin_tools, ResolvedPluginTool};

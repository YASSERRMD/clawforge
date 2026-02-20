pub mod installer;
pub mod manifest;
pub mod registry;
pub mod slots;

pub use installer::PluginInstaller;
pub use manifest::{PluginHookEntry, PluginManifest, PluginPermissions, PluginToolSlot};
pub use registry::PluginRegistry;
pub use slots::{collect_plugin_tools, ResolvedPluginTool};

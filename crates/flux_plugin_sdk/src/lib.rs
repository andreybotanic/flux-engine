//! Public Rust-first SDK for FluxEngine runtime plugins.

mod api;
mod descriptors;
mod dispatch_state_builder;
mod error;
mod events;
mod ids;
mod overlay_graph;
mod plugin;
mod registrar;
mod runtime_host;
mod scope;

pub use api::*;
pub use descriptors::*;
pub use error::PluginError;
pub use events::*;
pub use ids::*;
pub use overlay_graph::*;
pub use plugin::{Plugin, PluginInit};
pub use registrar::{Handler, Registrar};

pub(crate) use dispatch_state_builder::DispatchStateBuilder;

#[doc(hidden)]
pub mod __private {
    pub use crate::events::{AbiEventPayload, BuiltinEventPayload};
    pub use crate::plugin::{BuiltinPluginRuntime, PluginRuntime};
    pub use crate::registrar::MemoryRegistration;
    pub use crate::runtime_host::{
        RuntimeHostBinding, RuntimeHostFns, RuntimeSaveChunkData, RuntimeTimeSnapshot,
    };
    pub use crate::scope::DispatchState;
    pub use flux_plugin_abi::{
        FluxHostApi, FluxPluginDispatchFn, FluxPluginHandle, FluxPluginRegisterFn, FluxRegistrar,
        FluxRuntimeHost, FluxStatus, ENGINE_PLUGIN_API_VERSION_VALUE,
        FLUX_PLUGIN_API_VERSION_EXPORT_NAME, FLUX_PLUGIN_CREATE_EXPORT_NAME,
        FLUX_PLUGIN_DESTROY_EXPORT_NAME, FLUX_PLUGIN_DISPATCH_EXPORT_NAME,
        FLUX_PLUGIN_REGISTER_EXPORT_NAME,
    };
}

/// Declares one runtime plugin and generates the hidden ABI glue.
#[macro_export]
macro_rules! declare_plugin {
    ($plugin_ty:ty) => {
        #[no_mangle]
        pub extern "C" fn flux_plugin_api_version() -> u32 {
            $crate::__private::ENGINE_PLUGIN_API_VERSION_VALUE
        }

        #[no_mangle]
        pub unsafe extern "C" fn flux_plugin_create(
            host: *const $crate::__private::FluxHostApi,
            out_plugin: *mut *mut $crate::__private::FluxPluginHandle,
        ) -> $crate::__private::FluxStatus {
            $crate::__private::PluginRuntime::<$plugin_ty>::create(host, out_plugin)
        }

        #[no_mangle]
        pub unsafe extern "C" fn flux_plugin_register(
            plugin: *mut $crate::__private::FluxPluginHandle,
            registrar: *mut $crate::__private::FluxRegistrar,
        ) -> $crate::__private::FluxStatus {
            $crate::__private::PluginRuntime::<$plugin_ty>::register(plugin, registrar)
        }

        #[no_mangle]
        pub unsafe extern "C" fn flux_plugin_dispatch(
            plugin: *mut $crate::__private::FluxPluginHandle,
            event_kind: u32,
            payload: *const u8,
            payload_len: usize,
            host: *mut $crate::__private::FluxRuntimeHost,
        ) -> $crate::__private::FluxStatus {
            $crate::__private::PluginRuntime::<$plugin_ty>::dispatch(
                plugin,
                event_kind,
                payload,
                payload_len,
                host,
            )
        }

        #[no_mangle]
        pub unsafe extern "C" fn flux_plugin_destroy(
            plugin: *mut $crate::__private::FluxPluginHandle,
        ) {
            $crate::__private::PluginRuntime::<$plugin_ty>::destroy(plugin)
        }
    };
}

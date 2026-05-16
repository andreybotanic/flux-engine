use std::marker::PhantomData;

use flux_plugin_abi::{FluxHostApi, FluxPluginHandle, FluxRegistrar, FluxRuntimeHost, FluxStatus};

use crate::{
    registrar::MemoryRegistration,
    runtime_host::{abi_runtime_host_binding, RuntimeHostBinding},
    scope, BuiltinEventPayload, EntityApi, GasApi, InputApi, LoggerApi, OverlayApi,
    PluginApiVersion, PluginError, PluginEvent, PluginId, PluginPaths, Registrar,
    RegistrationPhase, SaveApi, TimeApi, UiApi, WorldApi,
};

/// Public runtime plugin contract implemented by plugin authors.
pub trait Plugin: Sized + 'static {
    /// Creates one plugin instance.
    fn new(init: PluginInit) -> Result<Self, PluginError>;

    /// Legacy registration entrypoint.
    ///
    /// This method remains the default content-registration callback and keeps
    /// backward compatibility for plugins that implemented only one phase.
    fn register(&mut self, registrar: &mut Registrar<Self>) -> Result<(), PluginError>;

    /// Registers plugin-owned entity categories.
    fn register_categories(&mut self, _registrar: &mut Registrar<Self>) -> Result<(), PluginError> {
        Ok(())
    }

    /// Registers plugin-owned content and event subscriptions.
    fn register_content(&mut self, registrar: &mut Registrar<Self>) -> Result<(), PluginError> {
        self.register(registrar)
    }
}

/// Data available to one plugin during construction.
#[derive(Clone, Debug)]
pub struct PluginInit {
    plugin_id: PluginId,
    engine_version: String,
    engine_api_version: PluginApiVersion,
    paths: PluginPaths,
    world: WorldApi,
    entities: EntityApi,
    gases: GasApi,
    ui: UiApi,
    overlays: OverlayApi,
    save: SaveApi,
    time: TimeApi,
    input: InputApi,
    logger: LoggerApi,
}

impl PluginInit {
    pub(crate) fn from_host(host: &FluxHostApi) -> Result<Self, PluginError> {
        Ok(Self::new(
            PluginId::parse(&scope::parse_utf8(host.plugin_id, "plugin_id")?)?,
            scope::parse_utf8(host.engine_version, "engine_version")?,
            PluginApiVersion(host.api_version),
            PluginPaths {
                plugin_root: scope::parse_utf8_path(host.plugin_root, "plugin_root")?,
                config_root: scope::parse_utf8_path(host.config_root, "config_root")?,
                assets_root: scope::parse_utf8_path(host.assets_root, "assets_root")?,
            },
        ))
    }

    pub(crate) fn new(
        plugin_id: PluginId,
        engine_version: String,
        engine_api_version: PluginApiVersion,
        paths: PluginPaths,
    ) -> Self {
        Self {
            plugin_id,
            engine_version,
            engine_api_version,
            paths,
            world: WorldApi,
            entities: EntityApi,
            gases: GasApi,
            ui: UiApi,
            overlays: OverlayApi,
            save: SaveApi,
            time: TimeApi,
            input: InputApi,
            logger: LoggerApi,
        }
    }

    /// Returns the plugin id assigned by the host.
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Returns the plugin root directory.
    pub fn plugin_root(&self) -> &std::path::Path {
        self.paths.plugin_root()
    }

    /// Returns the config root directory.
    pub fn config_root(&self) -> &std::path::Path {
        self.paths.config_root()
    }

    /// Returns the assets root directory.
    pub fn assets_root(&self) -> &std::path::Path {
        self.paths.assets_root()
    }

    /// Returns the engine ABI version.
    pub fn engine_api_version(&self) -> PluginApiVersion {
        self.engine_api_version
    }

    /// Returns the engine version string.
    pub fn engine_version(&self) -> &str {
        &self.engine_version
    }

    /// Returns a world API proxy.
    pub fn world_api(&self) -> WorldApi {
        self.world
    }

    /// Returns an entity API proxy.
    pub fn entity_api(&self) -> EntityApi {
        self.entities
    }

    /// Returns a gas API proxy.
    pub fn gas_api(&self) -> GasApi {
        self.gases
    }

    /// Returns a UI API proxy.
    pub fn ui_api(&self) -> UiApi {
        self.ui
    }

    /// Returns an overlay API proxy.
    pub fn overlay_api(&self) -> OverlayApi {
        self.overlays
    }

    /// Returns a save API proxy.
    pub fn save_api(&self) -> SaveApi {
        self.save
    }

    /// Returns a time API proxy.
    pub fn time_api(&self) -> TimeApi {
        self.time
    }

    /// Returns an input API proxy.
    pub fn input_api(&self) -> InputApi {
        self.input
    }

    /// Returns a logger API proxy.
    pub fn logger_api(&self) -> LoggerApi {
        self.logger
    }

    /// Writes one error log message during plugin creation.
    pub fn log_error(&self, message: impl Into<String>) {
        let _ = self.logger.error(message.into());
    }

    /// Writes one warning log message during plugin creation.
    pub fn log_warn(&self, message: impl Into<String>) {
        let _ = self.logger.warn(message.into());
    }

    /// Writes one info log message during plugin creation.
    pub fn log_info(&self, message: impl Into<String>) {
        let _ = self.logger.info(message.into());
    }
}

struct DispatchRegistration<P> {
    event_kind: PluginEvent,
    handler: *const (),
    abi_dispatch: unsafe fn(&mut P, *const (), *const u8, usize) -> Result<(), PluginError>,
    builtin_dispatch:
        unsafe fn(&mut P, *const (), &dyn BuiltinEventPayload) -> Result<(), PluginError>,
}

/// Hidden runtime wrapper stored behind the opaque ABI handle.
pub struct PluginRuntime<P: Plugin> {
    plugin_id: PluginId,
    plugin: P,
    handlers: Vec<DispatchRegistration<P>>,
    known_entity_categories: Vec<crate::EntityCategoryRef>,
    _marker: PhantomData<P>,
}

impl<P: Plugin> PluginRuntime<P> {
    /// Creates one plugin instance through the hidden ABI bridge.
    pub unsafe fn create(
        host: *const FluxHostApi,
        out_plugin: *mut *mut FluxPluginHandle,
    ) -> FluxStatus {
        if host.is_null() || out_plugin.is_null() {
            return FluxStatus::INVALID_ARGUMENT;
        }
        let host = &*host;
        let created = scope::with_init_scope(host, || -> Result<Self, PluginError> {
            let init = PluginInit::from_host(host)?;
            let plugin_id = init.plugin_id().clone();
            let plugin = P::new(init)?;
            Ok(Self {
                plugin_id,
                plugin,
                handlers: Vec::new(),
                known_entity_categories: Vec::new(),
                _marker: PhantomData,
            })
        });
        match created {
            Ok(runtime) => {
                *out_plugin = Box::into_raw(Box::new(runtime)).cast::<FluxPluginHandle>();
                FluxStatus::OK
            }
            Err(error) => {
                let _ = scope::write_log(1, &error.to_string());
                FluxStatus::FAILED
            }
        }
    }

    /// Runs the hidden registration pass and stores the dispatch table.
    pub unsafe fn register(
        plugin: *mut FluxPluginHandle,
        registrar: *mut FluxRegistrar,
    ) -> FluxStatus {
        if plugin.is_null() || registrar.is_null() {
            return FluxStatus::INVALID_ARGUMENT;
        }
        let runtime = &mut *plugin.cast::<Self>();
        let registrar = &mut *registrar;
        let mut public_registrar = Registrar::new(runtime.plugin_id.clone(), registrar);
        public_registrar.seed_entity_categories(&runtime.known_entity_categories);
        match run_plugin_registration_phase(&mut runtime.plugin, &mut public_registrar) {
            Ok(()) => {
                runtime.known_entity_categories = public_registrar.entity_categories();
                runtime.handlers = public_registrar
                    .finish()
                    .into_iter()
                    .map(|handler| DispatchRegistration {
                        event_kind: handler.event_kind,
                        handler: handler.handler,
                        abi_dispatch: handler.abi_dispatch,
                        builtin_dispatch: handler.builtin_dispatch,
                    })
                    .collect();
                FluxStatus::OK
            }
            Err(error) => {
                let _ = scope::write_log(1, &error.to_string());
                FluxStatus::FAILED
            }
        }
    }

    /// Dispatches one subscribed event into the plugin instance.
    pub unsafe fn dispatch(
        plugin: *mut FluxPluginHandle,
        event_kind: u32,
        payload: *const u8,
        payload_len: usize,
        host: *mut FluxRuntimeHost,
    ) -> FluxStatus {
        if plugin.is_null() || host.is_null() {
            return FluxStatus::INVALID_ARGUMENT;
        }
        let runtime = &mut *plugin.cast::<Self>();
        let Some(event_kind) = PluginEvent::from_raw(event_kind) else {
            return FluxStatus::INVALID_ARGUMENT;
        };
        let Some(handler) = runtime
            .handlers
            .iter()
            .find(|handler| handler.event_kind == event_kind)
        else {
            return FluxStatus::OK;
        };
        let state = crate::DispatchStateBuilder::build(event_kind, payload, payload_len);
        let Some(runtime_host) = abi_runtime_host_binding(host) else {
            return FluxStatus::INVALID_ARGUMENT;
        };
        let result = scope::with_runtime_scope(runtime_host, state, || {
            (handler.abi_dispatch)(&mut runtime.plugin, handler.handler, payload, payload_len)
        });
        match result {
            Ok(()) => FluxStatus::OK,
            Err(error) => {
                let _ = scope::write_log(1, &error.to_string());
                FluxStatus::FAILED
            }
        }
    }

    /// Destroys one plugin runtime instance.
    pub unsafe fn destroy(plugin: *mut FluxPluginHandle) {
        if plugin.is_null() {
            return;
        }
        let _ = Box::from_raw(plugin.cast::<Self>());
    }
}

/// Hidden in-process runtime wrapper used by built-in plugins.
pub struct BuiltinPluginRuntime<P: Plugin> {
    plugin_id: PluginId,
    plugin: P,
    handlers: Vec<DispatchRegistration<P>>,
    registration: MemoryRegistration,
}

unsafe impl<P: Plugin> Send for BuiltinPluginRuntime<P> {}
unsafe impl<P: Plugin> Sync for BuiltinPluginRuntime<P> {}

impl<P: Plugin> BuiltinPluginRuntime<P> {
    /// Creates one built-in plugin instance without ABI exports or DLL loading.
    pub fn create(
        plugin_id: PluginId,
        engine_version: String,
        engine_api_version: PluginApiVersion,
        paths: PluginPaths,
    ) -> Result<Self, PluginError> {
        let init = PluginInit::new(plugin_id.clone(), engine_version, engine_api_version, paths);
        let plugin = P::new(init)?;
        Ok(Self {
            plugin_id,
            plugin,
            handlers: Vec::new(),
            registration: MemoryRegistration::default(),
        })
    }

    /// Runs plugin registration into the in-memory built-in registrar.
    pub fn register(&mut self) -> Result<(), PluginError> {
        let mut registration = MemoryRegistration::default();
        {
            let mut registrar = Registrar::new_memory(
                self.plugin_id.clone(),
                &mut registration,
                RegistrationPhase::Categories,
            );
            run_plugin_registration_phase(&mut self.plugin, &mut registrar)?;
        }
        let mut registrar = Registrar::new_memory(
            self.plugin_id.clone(),
            &mut registration,
            RegistrationPhase::Content,
        );
        run_plugin_registration_phase(&mut self.plugin, &mut registrar)?;
        self.handlers = registrar
            .finish()
            .into_iter()
            .map(|handler| DispatchRegistration {
                event_kind: handler.event_kind,
                handler: handler.handler,
                abi_dispatch: handler.abi_dispatch,
                builtin_dispatch: handler.builtin_dispatch,
            })
            .collect();
        self.registration = registration;
        Ok(())
    }

    /// Returns the stable plugin id owned by this built-in runtime instance.
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Returns the built-in registration snapshot collected during `register`.
    pub fn registration(&self) -> &MemoryRegistration {
        &self.registration
    }

    /// Dispatches one typed built-in event into the plugin instance.
    pub fn dispatch(
        &mut self,
        event_kind: PluginEvent,
        payload: &dyn BuiltinEventPayload,
        runtime_host: RuntimeHostBinding,
        state: scope::DispatchState,
    ) -> Result<(), PluginError> {
        let Some(handler) = self
            .handlers
            .iter()
            .find(|handler| handler.event_kind == event_kind)
        else {
            return Ok(());
        };
        scope::with_runtime_scope(runtime_host, state, || unsafe {
            (handler.builtin_dispatch)(&mut self.plugin, handler.handler, payload)
        })
    }
}

fn run_plugin_registration_phase<P: Plugin>(
    plugin: &mut P,
    registrar: &mut Registrar<P>,
) -> Result<(), PluginError> {
    if registrar.is_category_phase() {
        plugin.register_categories(registrar)
    } else {
        plugin.register_content(registrar)
    }
}

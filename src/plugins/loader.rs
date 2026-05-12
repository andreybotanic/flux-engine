use std::{
    ffi::c_void,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    ptr,
    time::{SystemTime, UNIX_EPOCH},
};

use libloading::Library;
use zip::ZipArchive;

use crate::plugins::{
    abi::{
        build_host_api, build_registrar, event_kind_from_abi, FluxEntityDescriptor,
        FluxGasSubstanceDescriptor, FluxOverlayDescriptor, FluxPluginApiVersionFn,
        FluxPluginCreateFn, FluxPluginDestroyFn, FluxPluginDispatchFn, FluxPluginHandle,
        FluxPluginRegisterFn, FluxSaveChunkDescriptor, FluxStatus, FluxSubscriptionDescriptor,
        FluxToolDescriptor, FluxUtf8Slice,
        FLUX_PLUGIN_API_VERSION_EXPORT_NAME,
        FLUX_PLUGIN_CREATE_EXPORT_NAME, FLUX_PLUGIN_DESTROY_EXPORT_NAME,
        FLUX_PLUGIN_DISPATCH_EXPORT_NAME, FLUX_PLUGIN_REGISTER_EXPORT_NAME,
    },
    api::{
        render_api::OverlayRenderPolicy,
        runtime::{RuntimeOverlayDescriptor, SaveChunkDescriptor},
        ui_api::ToolDescriptor,
    },
    diagnostics::PluginContractError,
    id::{PluginApiVersion, ENGINE_PLUGIN_API_VERSION},
    manifest::PluginManifest,
    registration::{PluginRuntimeRegistration, PluginSubscriptionRegistration},
    runtime_dll::RuntimeDllPlugin,
    source::{
        fingerprint_expanded_plugin_root, fingerprint_packaged_plugin_archive,
        resolve_plugin_layout, validate_archive_entry_path, ExpandedPluginSource,
        PackagedPluginSource, PluginSourceFingerprint, PluginSourceKind, MANIFEST_FILE_NAME,
    },
    ContentId, LoadedPluginMetadata, PluginId, SubstanceDefinition, SubstanceId, SubstanceRegistry,
};

/// Parsed packaged plugin candidate before DLL handshake validation.
#[derive(Clone, Debug)]
pub(crate) struct PackagedPluginCandidate {
    pub source: PackagedPluginSource,
    pub manifest: PluginManifest,
}

/// Successfully validated packaged plugin contract.
#[derive(Clone, Debug)]
pub(crate) struct ValidatedPackagedPlugin {
    pub source: PackagedPluginSource,
    pub manifest: PluginManifest,
    pub registration: PluginRuntimeRegistration,
    pub fingerprint: PluginSourceFingerprint,
}

/// Parsed expanded plugin candidate before DLL handshake validation.
#[derive(Clone, Debug)]
pub(crate) struct ExpandedPluginCandidate {
    pub source: ExpandedPluginSource,
    pub manifest: PluginManifest,
}

/// Successfully validated expanded plugin contract.
#[derive(Clone, Debug)]
pub(crate) struct ValidatedExpandedPlugin {
    pub source: ExpandedPluginSource,
    pub manifest: PluginManifest,
    pub registration: PluginRuntimeRegistration,
    pub fingerprint: PluginSourceFingerprint,
}

/// Reads and validates only the manifest layer of one packaged plugin archive.
pub(crate) fn read_packaged_plugin_candidate(
    archive_path: &Path,
) -> Result<PackagedPluginCandidate, PluginContractError> {
    let source = PackagedPluginSource::new(archive_path.to_path_buf());
    let file = File::open(archive_path).map_err(|error| {
        PluginContractError::Io(format!(
            "failed to open plugin archive '{}': {}",
            archive_path.display(),
            error
        ))
    })?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        PluginContractError::Archive(format!(
            "failed to read ZIP archive '{}': {}",
            archive_path.display(),
            error
        ))
    })?;

    let mut manifest_bytes = None;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            PluginContractError::Archive(format!(
                "failed to read ZIP entry #{} from '{}': {}",
                index,
                archive_path.display(),
                error
            ))
        })?;
        let entry_path = validate_archive_entry_path(entry.name())?;
        if entry_path == Path::new(MANIFEST_FILE_NAME) {
            if entry.is_dir() {
                return Err(PluginContractError::Archive(
                    "root manifest.toml must be a file".to_string(),
                ));
            }
            if manifest_bytes.is_some() {
                return Err(PluginContractError::Archive(
                    "archive contains multiple root manifest.toml files".to_string(),
                ));
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).map_err(|error| {
                PluginContractError::Archive(format!(
                    "failed to read manifest.toml from '{}': {}",
                    archive_path.display(),
                    error
                ))
            })?;
            manifest_bytes = Some(bytes);
        }
    }

    let manifest_bytes = manifest_bytes.ok_or_else(|| {
        PluginContractError::Archive("archive is missing root manifest.toml".to_string())
    })?;
    let manifest = PluginManifest::from_bytes(&manifest_bytes)?;

    Ok(PackagedPluginCandidate { source, manifest })
}

/// Reads and validates only the manifest layer of one expanded plugin directory.
pub(crate) fn read_expanded_plugin_candidate(
    root_dir: &Path,
) -> Result<ExpandedPluginCandidate, PluginContractError> {
    let source = ExpandedPluginSource::new(root_dir.to_path_buf());
    let manifest_path = root_dir.join(MANIFEST_FILE_NAME);
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
        PluginContractError::Io(format!(
            "failed to read expanded manifest '{}': {}",
            manifest_path.display(),
            error
        ))
    })?;
    let manifest = PluginManifest::from_bytes(&manifest_bytes)?;

    Ok(ExpandedPluginCandidate { source, manifest })
}

/// Validates one packaged plugin archive using the same contract as runtime discovery.
pub fn validate_packaged_plugin_archive(
    archive_path: &Path,
) -> Result<(PluginManifest, PluginRuntimeRegistration), PluginContractError> {
    let candidate = read_packaged_plugin_candidate(archive_path)?;
    let validated = validate_packaged_plugin_candidate(&candidate)?;
    Ok((validated.manifest, validated.registration))
}

/// Validates one expanded plugin directory using the same contract as runtime discovery.
pub fn validate_expanded_plugin_root(
    root_dir: &Path,
) -> Result<(PluginManifest, PluginRuntimeRegistration), PluginContractError> {
    let candidate = read_expanded_plugin_candidate(root_dir)?;
    let validated = validate_expanded_plugin_candidate(&candidate)?;
    Ok((validated.manifest, validated.registration))
}

/// Validates archive extraction and DLL ABI handshake for one packaged plugin.
pub(crate) fn validate_packaged_plugin_candidate(
    candidate: &PackagedPluginCandidate,
) -> Result<ValidatedPackagedPlugin, PluginContractError> {
    if candidate.manifest.api_version != ENGINE_PLUGIN_API_VERSION {
        return Err(PluginContractError::Manifest(format!(
            "unsupported api_version '{}'; engine supports '{}'",
            candidate.manifest.api_version, ENGINE_PLUGIN_API_VERSION
        )));
    }

    let extracted_root = extract_packaged_plugin_to_cache(candidate)?;
    let result = validate_plugin_root(&candidate.manifest, &extracted_root);
    let _ = fs::remove_dir_all(&extracted_root);
    let registration = result?;
    validate_runtime_registration(&candidate.manifest, &registration)?;
    let fingerprint = fingerprint_packaged_plugin_archive(candidate.source.archive_path())?;

    Ok(ValidatedPackagedPlugin {
        source: candidate.source.clone(),
        manifest: candidate.manifest.clone(),
        registration,
        fingerprint,
    })
}

/// Validates cache-copy extraction and DLL ABI handshake for one expanded plugin.
pub(crate) fn validate_expanded_plugin_candidate(
    candidate: &ExpandedPluginCandidate,
) -> Result<ValidatedExpandedPlugin, PluginContractError> {
    if candidate.manifest.api_version != ENGINE_PLUGIN_API_VERSION {
        return Err(PluginContractError::Manifest(format!(
            "unsupported api_version '{}'; engine supports '{}'",
            candidate.manifest.api_version, ENGINE_PLUGIN_API_VERSION
        )));
    }

    let cached_root = copy_expanded_plugin_to_cache(candidate)?;
    let result = validate_plugin_root(&candidate.manifest, &cached_root);
    let _ = fs::remove_dir_all(&cached_root);
    let registration = result?;
    validate_runtime_registration(&candidate.manifest, &registration)?;
    let fingerprint =
        fingerprint_expanded_plugin_root(candidate.source.root_dir(), &candidate.manifest)?;

    Ok(ValidatedExpandedPlugin {
        source: candidate.source.clone(),
        manifest: candidate.manifest.clone(),
        registration,
        fingerprint,
    })
}

/// Validates plugin-owned content registration against manifest-level constraints.
pub fn validate_runtime_registration(
    manifest: &PluginManifest,
    registration: &PluginRuntimeRegistration,
) -> Result<(), PluginContractError> {
    validate_subscription_registration(&registration.subscriptions)?;
    if !manifest.content && !registration.gas_substances.is_empty() {
        return Err(PluginContractError::Abi(format!(
            "plugin '{}' has content = false but registered runtime content",
            manifest.id
        )));
    }
    if registration.gas_substances.is_empty() {
        return Ok(());
    }
    SubstanceRegistry::new(registration.gas_substances.clone()).map_err(|error| {
        PluginContractError::Abi(format!(
            "plugin '{}' registered invalid gas substances: {}",
            manifest.id, error
        ))
    })?;
    Ok(())
}

fn validate_subscription_registration(
    subscriptions: &[PluginSubscriptionRegistration],
) -> Result<(), PluginContractError> {
    use std::collections::BTreeSet;

    let mut event_kinds = BTreeSet::new();
    for subscription in subscriptions {
        if !event_kinds.insert(subscription.event_kind) {
            return Err(PluginContractError::Abi(format!(
                "duplicate event subscription for {:?}",
                subscription.event_kind
            )));
        }
    }
    Ok(())
}

/// Creates a live DLL plugin instance for runtime event dispatch.
pub fn instantiate_runtime_plugin(
    plugin: &LoadedPluginMetadata,
) -> Result<RuntimeDllPlugin, PluginContractError> {
    let manifest = plugin.manifest.as_ref().ok_or_else(|| {
        PluginContractError::Manifest(format!(
            "plugin '{}' has no runtime manifest",
            plugin.plugin_id
        ))
    })?;
    let source_path = plugin.source_path.as_ref().ok_or_else(|| {
        PluginContractError::Manifest(format!(
            "plugin '{}' has no runtime source path",
            plugin.plugin_id
        ))
    })?;

    let cache_root = match plugin.source_kind {
        PluginSourceKind::Packaged => {
            let candidate = PackagedPluginCandidate {
                source: PackagedPluginSource::new(source_path.clone()),
                manifest: manifest.clone(),
            };
            extract_packaged_plugin_to_cache(&candidate)?
        }
        PluginSourceKind::Dev => {
            let candidate = ExpandedPluginCandidate {
                source: ExpandedPluginSource::new(source_path.clone()),
                manifest: manifest.clone(),
            };
            copy_expanded_plugin_to_cache(&candidate)?
        }
        PluginSourceKind::Builtin => {
            return Err(PluginContractError::Manifest(
                "builtin plugin does not have a runtime DLL".to_string(),
            ));
        }
    };

    let result = instantiate_runtime_plugin_from_root(plugin, manifest, &cache_root);
    if result.is_err() {
        let _ = fs::remove_dir_all(&cache_root);
    }
    result
}

fn instantiate_runtime_plugin_from_root(
    plugin: &LoadedPluginMetadata,
    manifest: &PluginManifest,
    plugin_root: &Path,
) -> Result<RuntimeDllPlugin, PluginContractError> {
    let resolved_paths = resolve_plugin_layout(plugin_root, manifest)?;
    let plugin_root_utf8 = path_to_utf8_string(&resolved_paths.root_dir);
    let config_root_utf8 = path_to_utf8_string(&resolved_paths.configs_dir);
    let assets_root_utf8 = path_to_utf8_string(&resolved_paths.assets_dir);
    let mut host_error_message = String::new();

    unsafe {
        let library = Library::new(&resolved_paths.dll_path).map_err(|error| {
            PluginContractError::Dll(format!(
                "failed to load DLL '{}': {}",
                manifest.dll.display(),
                error
            ))
        })?;
        let api_version_fn = load_symbol::<FluxPluginApiVersionFn>(
            &library,
            FLUX_PLUGIN_API_VERSION_EXPORT_NAME,
            "flux_plugin_api_version",
        )?;
        let create_fn = load_symbol::<FluxPluginCreateFn>(
            &library,
            FLUX_PLUGIN_CREATE_EXPORT_NAME,
            "flux_plugin_create",
        )?;
        let register_fn = load_symbol::<FluxPluginRegisterFn>(
            &library,
            FLUX_PLUGIN_REGISTER_EXPORT_NAME,
            "flux_plugin_register",
        )?;
        let dispatch_fn = load_symbol::<FluxPluginDispatchFn>(
            &library,
            FLUX_PLUGIN_DISPATCH_EXPORT_NAME,
            "flux_plugin_dispatch",
        )?;
        let destroy_fn = load_symbol::<FluxPluginDestroyFn>(
            &library,
            FLUX_PLUGIN_DESTROY_EXPORT_NAME,
            "flux_plugin_destroy",
        )?;

        let exported_api_version = PluginApiVersion::new(api_version_fn());
        if exported_api_version != ENGINE_PLUGIN_API_VERSION {
            return Err(PluginContractError::Abi(format!(
                "DLL API version '{}' does not match engine API '{}'",
                exported_api_version, ENGINE_PLUGIN_API_VERSION
            )));
        }

        let host_api = build_host_api(
            FluxUtf8Slice::from_str(plugin.plugin_id.as_str()),
            FluxUtf8Slice::from_str(env!("CARGO_PKG_VERSION")),
            FluxUtf8Slice::from_str(&plugin_root_utf8),
            FluxUtf8Slice::from_str(&config_root_utf8),
            FluxUtf8Slice::from_str(&assets_root_utf8),
            Some(write_host_log_message),
            (&mut host_error_message as *mut String).cast::<c_void>(),
        );
        let mut plugin_handle: *mut FluxPluginHandle = ptr::null_mut();
        let create_status = create_fn(&host_api, &mut plugin_handle);
        if !create_status.is_ok() {
            if !plugin_handle.is_null() {
                destroy_fn(plugin_handle);
            }
            return Err(status_error(
                "flux_plugin_create",
                create_status,
                &host_error_message,
                PluginContractError::Abi,
            ));
        }
        if plugin_handle.is_null() {
            return Err(PluginContractError::Abi(
                "flux_plugin_create returned OK but plugin handle is null".to_string(),
            ));
        }

        let mut registrar = build_registrar(
            Some(register_noop_gas_substance_callback),
            Some(register_noop_entity_callback),
            Some(register_noop_tool_callback),
            None,
            Some(register_noop_overlay_callback),
            Some(register_noop_save_chunk_callback),
            Some(register_noop_subscription_callback),
            ptr::null_mut(),
        );
        let register_status = register_fn(plugin_handle, &mut registrar);
        if !register_status.is_ok() {
            destroy_fn(plugin_handle);
            return Err(status_error(
                "flux_plugin_register",
                register_status,
                &host_error_message,
                PluginContractError::Abi,
            ));
        }

        Ok(RuntimeDllPlugin {
            plugin_id: plugin.plugin_id.clone(),
            handle: plugin_handle,
            dispatch_fn,
            destroy_fn,
            cache_root: plugin_root.to_path_buf(),
            library,
        })
    }
}

fn extract_packaged_plugin_to_cache(
    candidate: &PackagedPluginCandidate,
) -> Result<PathBuf, PluginContractError> {
    let archive_path = candidate.source.archive_path();
    let file = File::open(archive_path).map_err(|error| {
        PluginContractError::Io(format!(
            "failed to reopen plugin archive '{}': {}",
            archive_path.display(),
            error
        ))
    })?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        PluginContractError::Archive(format!(
            "failed to read ZIP archive '{}': {}",
            archive_path.display(),
            error
        ))
    })?;

    let extraction_root = cache_generation_root(&candidate.manifest.id)?;
    fs::create_dir_all(&extraction_root).map_err(|error| {
        PluginContractError::Io(format!(
            "failed to create plugin cache directory '{}': {}",
            extraction_root.display(),
            error
        ))
    })?;

    let extraction_result = (|| -> Result<(), PluginContractError> {
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|error| {
                PluginContractError::Archive(format!(
                    "failed to read ZIP entry #{} from '{}': {}",
                    index,
                    archive_path.display(),
                    error
                ))
            })?;
            let relative_path = validate_archive_entry_path(entry.name())?;
            let destination = extraction_root.join(&relative_path);

            if entry.is_dir() {
                fs::create_dir_all(&destination).map_err(|error| {
                    PluginContractError::Io(format!(
                        "failed to create extracted directory '{}': {}",
                        destination.display(),
                        error
                    ))
                })?;
                continue;
            }

            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    PluginContractError::Io(format!(
                        "failed to create extracted parent directory '{}': {}",
                        parent.display(),
                        error
                    ))
                })?;
            }

            let mut output = File::create(&destination).map_err(|error| {
                PluginContractError::Io(format!(
                    "failed to create extracted file '{}': {}",
                    destination.display(),
                    error
                ))
            })?;
            std::io::copy(&mut entry, &mut output).map_err(|error| {
                PluginContractError::Io(format!(
                    "failed to extract '{}' into '{}': {}",
                    relative_path.display(),
                    destination.display(),
                    error
                ))
            })?;
            output.flush().map_err(|error| {
                PluginContractError::Io(format!(
                    "failed to flush extracted file '{}': {}",
                    destination.display(),
                    error
                ))
            })?;
        }

        Ok(())
    })();

    if extraction_result.is_err() {
        let _ = fs::remove_dir_all(&extraction_root);
    }

    extraction_result.map(|_| extraction_root)
}

fn copy_expanded_plugin_to_cache(
    candidate: &ExpandedPluginCandidate,
) -> Result<PathBuf, PluginContractError> {
    let cache_root = cache_generation_root(&candidate.manifest.id)?;
    copy_directory_recursive(candidate.source.root_dir(), &cache_root)?;
    Ok(cache_root)
}

fn copy_directory_recursive(
    source_dir: &Path,
    destination_dir: &Path,
) -> Result<(), PluginContractError> {
    fs::create_dir_all(destination_dir).map_err(|error| {
        PluginContractError::Io(format!(
            "failed to create plugin cache directory '{}': {}",
            destination_dir.display(),
            error
        ))
    })?;

    for entry in fs::read_dir(source_dir).map_err(|error| {
        PluginContractError::Io(format!(
            "failed to read expanded plugin directory '{}': {}",
            source_dir.display(),
            error
        ))
    })? {
        let entry = entry.map_err(|error| {
            PluginContractError::Io(format!(
                "failed to read one expanded plugin entry from '{}': {}",
                source_dir.display(),
                error
            ))
        })?;
        let source_path = entry.path();
        let destination_path = destination_dir.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
            continue;
        }

        fs::copy(&source_path, &destination_path).map_err(|error| {
            PluginContractError::Io(format!(
                "failed to copy expanded plugin file '{}' into '{}': {}",
                source_path.display(),
                destination_path.display(),
                error
            ))
        })?;
    }

    Ok(())
}

fn validate_plugin_root(
    manifest: &PluginManifest,
    plugin_root: &Path,
) -> Result<PluginRuntimeRegistration, PluginContractError> {
    let resolved_paths = resolve_plugin_layout(plugin_root, manifest)?;
    let plugin_root_utf8 = path_to_utf8_string(&resolved_paths.root_dir);
    let config_root_utf8 = path_to_utf8_string(&resolved_paths.configs_dir);
    let assets_root_utf8 = path_to_utf8_string(&resolved_paths.assets_dir);
    let mut host_error_message = String::new();
    let mut registration_collector = PluginRegistrationCollector::new(manifest.id.clone());

    unsafe {
        let library = Library::new(&resolved_paths.dll_path).map_err(|error| {
            PluginContractError::Dll(format!(
                "failed to load DLL '{}': {}",
                manifest.dll.display(),
                error
            ))
        })?;
        let api_version_fn = load_symbol::<FluxPluginApiVersionFn>(
            &library,
            FLUX_PLUGIN_API_VERSION_EXPORT_NAME,
            "flux_plugin_api_version",
        )?;
        let create_fn = load_symbol::<FluxPluginCreateFn>(
            &library,
            FLUX_PLUGIN_CREATE_EXPORT_NAME,
            "flux_plugin_create",
        )?;
        let register_fn = load_symbol::<FluxPluginRegisterFn>(
            &library,
            FLUX_PLUGIN_REGISTER_EXPORT_NAME,
            "flux_plugin_register",
        )?;
        let destroy_fn = load_symbol::<FluxPluginDestroyFn>(
            &library,
            FLUX_PLUGIN_DESTROY_EXPORT_NAME,
            "flux_plugin_destroy",
        )?;

        let exported_api_version = PluginApiVersion::new(api_version_fn());
        if exported_api_version != ENGINE_PLUGIN_API_VERSION {
            return Err(PluginContractError::Abi(format!(
                "DLL API version '{}' does not match engine API '{}'",
                exported_api_version, ENGINE_PLUGIN_API_VERSION
            )));
        }

        let host_api = build_host_api(
            FluxUtf8Slice::from_str(manifest.id.as_str()),
            FluxUtf8Slice::from_str(env!("CARGO_PKG_VERSION")),
            FluxUtf8Slice::from_str(&plugin_root_utf8),
            FluxUtf8Slice::from_str(&config_root_utf8),
            FluxUtf8Slice::from_str(&assets_root_utf8),
            Some(write_host_log_message),
            (&mut host_error_message as *mut String).cast::<c_void>(),
        );
        let mut plugin_handle: *mut FluxPluginHandle = ptr::null_mut();
        let create_status = create_fn(&host_api, &mut plugin_handle);
        if !create_status.is_ok() {
            if !plugin_handle.is_null() {
                destroy_fn(plugin_handle);
            }
            return Err(status_error(
                "flux_plugin_create",
                create_status,
                &host_error_message,
                PluginContractError::Abi,
            ));
        }
        if plugin_handle.is_null() {
            return Err(PluginContractError::Abi(
                "flux_plugin_create returned OK but plugin handle is null".to_string(),
            ));
        }

        host_error_message.clear();
        let mut registrar = build_registrar(
            Some(register_gas_substance_callback),
            Some(register_entity_callback),
            Some(register_tool_callback),
            None,
            Some(register_overlay_callback),
            Some(register_save_chunk_callback),
            Some(register_subscription_callback),
            (&mut registration_collector as *mut PluginRegistrationCollector).cast::<c_void>(),
        );
        let register_status = register_fn(plugin_handle, &mut registrar);
        if !register_status.is_ok() {
            destroy_fn(plugin_handle);
            return Err(status_error(
                "flux_plugin_register",
                register_status,
                &host_error_message,
                PluginContractError::Abi,
            ));
        }
        destroy_fn(plugin_handle);
    }

    if let Some(error) = registration_collector.error_message {
        return Err(PluginContractError::Abi(error));
    }

    Ok(registration_collector.registration)
}

struct PluginRegistrationCollector {
    plugin_id: PluginId,
    registration: PluginRuntimeRegistration,
    error_message: Option<String>,
}

impl PluginRegistrationCollector {
    fn new(plugin_id: PluginId) -> Self {
        Self {
            plugin_id,
            registration: PluginRuntimeRegistration::default(),
            error_message: None,
        }
    }
}

unsafe extern "C" fn register_gas_substance_callback(
    context: *mut c_void,
    descriptor: *const FluxGasSubstanceDescriptor,
) -> FluxStatus {
    if context.is_null() || descriptor.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let collector = &mut *(context.cast::<PluginRegistrationCollector>());
    match gas_substance_from_abi(&collector.plugin_id, &*descriptor) {
        Ok(definition) => {
            collector.registration.gas_substances.push(definition);
            FluxStatus::OK
        }
        Err(error) => {
            collector.error_message = Some(error);
            FluxStatus::FAILED
        }
    }
}

unsafe extern "C" fn register_entity_callback(
    context: *mut c_void,
    descriptor: *const FluxEntityDescriptor,
) -> FluxStatus {
    if context.is_null() || descriptor.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let collector = &mut *(context.cast::<PluginRegistrationCollector>());
    match entity_descriptor_from_abi(&*descriptor) {
        Ok(descriptor) => {
            if collector
                .registration
                .entities
                .iter()
                .any(|registered| registered.id == descriptor.id)
            {
                collector.error_message = Some(format!(
                    "duplicate entity registration '{}'",
                    descriptor.id
                ));
                return FluxStatus::FAILED;
            }
            collector.registration.entities.push(descriptor);
            FluxStatus::OK
        }
        Err(error) => {
            collector.error_message = Some(error);
            FluxStatus::FAILED
        }
    }
}

unsafe extern "C" fn register_tool_callback(
    context: *mut c_void,
    descriptor: *const FluxToolDescriptor,
) -> FluxStatus {
    if context.is_null() || descriptor.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let collector = &mut *(context.cast::<PluginRegistrationCollector>());
    match tool_descriptor_from_abi(&*descriptor) {
        Ok(descriptor) => {
            collector.registration.tools.push(descriptor);
            FluxStatus::OK
        }
        Err(error) => {
            collector.error_message = Some(error);
            FluxStatus::FAILED
        }
    }
}

unsafe extern "C" fn register_overlay_callback(
    context: *mut c_void,
    descriptor: *const FluxOverlayDescriptor,
) -> FluxStatus {
    if context.is_null() || descriptor.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let collector = &mut *(context.cast::<PluginRegistrationCollector>());
    match overlay_descriptor_from_abi(&collector.plugin_id, &*descriptor) {
        Ok(descriptor) => {
            collector.registration.overlays.push(descriptor);
            FluxStatus::OK
        }
        Err(error) => {
            collector.error_message = Some(error);
            FluxStatus::FAILED
        }
    }
}

unsafe extern "C" fn register_save_chunk_callback(
    context: *mut c_void,
    descriptor: *const FluxSaveChunkDescriptor,
) -> FluxStatus {
    if context.is_null() || descriptor.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let collector = &mut *(context.cast::<PluginRegistrationCollector>());
    match save_chunk_descriptor_from_abi(&collector.plugin_id, &*descriptor) {
        Ok(descriptor) => {
            collector.registration.save_chunks.push(descriptor);
            FluxStatus::OK
        }
        Err(error) => {
            collector.error_message = Some(error);
            FluxStatus::FAILED
        }
    }
}

unsafe extern "C" fn register_subscription_callback(
    context: *mut c_void,
    descriptor: *const FluxSubscriptionDescriptor,
) -> FluxStatus {
    if context.is_null() || descriptor.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let collector = &mut *(context.cast::<PluginRegistrationCollector>());
    match subscription_from_abi(&*descriptor) {
        Ok(subscription) => {
            if collector
                .registration
                .subscriptions
                .iter()
                .any(|registered| registered.event_kind == subscription.event_kind)
            {
                collector.error_message = Some(format!(
                    "duplicate event subscription for {:?}",
                    subscription.event_kind
                ));
                return FluxStatus::FAILED;
            }
            collector.registration.subscriptions.push(subscription);
            FluxStatus::OK
        }
        Err(error) => {
            collector.error_message = Some(error);
            FluxStatus::FAILED
        }
    }
}

fn subscription_from_abi(
    descriptor: &FluxSubscriptionDescriptor,
) -> Result<PluginSubscriptionRegistration, String> {
    let event_kind = event_kind_from_abi(descriptor.event_kind)
        .ok_or_else(|| format!("unknown plugin event kind {}", descriptor.event_kind))?;
    Ok(PluginSubscriptionRegistration { event_kind })
}

unsafe extern "C" fn register_noop_gas_substance_callback(
    _context: *mut c_void,
    descriptor: *const FluxGasSubstanceDescriptor,
) -> FluxStatus {
    if descriptor.is_null() {
        FluxStatus::INVALID_ARGUMENT
    } else {
        FluxStatus::OK
    }
}

unsafe extern "C" fn register_noop_entity_callback(
    _context: *mut c_void,
    descriptor: *const FluxEntityDescriptor,
) -> FluxStatus {
    if descriptor.is_null() {
        FluxStatus::INVALID_ARGUMENT
    } else {
        FluxStatus::OK
    }
}

unsafe extern "C" fn register_noop_tool_callback(
    _context: *mut c_void,
    descriptor: *const FluxToolDescriptor,
) -> FluxStatus {
    if descriptor.is_null() {
        FluxStatus::INVALID_ARGUMENT
    } else {
        FluxStatus::OK
    }
}

unsafe extern "C" fn register_noop_overlay_callback(
    _context: *mut c_void,
    descriptor: *const FluxOverlayDescriptor,
) -> FluxStatus {
    if descriptor.is_null() {
        FluxStatus::INVALID_ARGUMENT
    } else {
        FluxStatus::OK
    }
}

unsafe extern "C" fn register_noop_save_chunk_callback(
    _context: *mut c_void,
    descriptor: *const FluxSaveChunkDescriptor,
) -> FluxStatus {
    if descriptor.is_null() {
        FluxStatus::INVALID_ARGUMENT
    } else {
        FluxStatus::OK
    }
}

unsafe extern "C" fn register_noop_subscription_callback(
    _context: *mut c_void,
    descriptor: *const FluxSubscriptionDescriptor,
) -> FluxStatus {
    if descriptor.is_null() {
        FluxStatus::INVALID_ARGUMENT
    } else {
        FluxStatus::OK
    }
}

unsafe fn entity_descriptor_from_abi(
    descriptor: &FluxEntityDescriptor,
) -> Result<flux_plugin_sdk::EntityDescriptor, String> {
    let id = read_abi_utf8(descriptor.id, "entity id")?;
    let label = read_abi_utf8(descriptor.label, "entity label")?;
    let icon_path = read_abi_utf8(descriptor.icon_path, "entity icon path")?;
    let silhouette_path = read_abi_utf8(descriptor.silhouette_path, "entity silhouette path")?;
    Ok(flux_plugin_sdk::EntityDescriptor {
        id: flux_plugin_sdk::ContentId::parse(&id)?,
        label,
        icon_path,
        silhouette_path: (!silhouette_path.trim().is_empty()).then_some(silhouette_path),
    })
}

unsafe fn tool_descriptor_from_abi(
    descriptor: &FluxToolDescriptor,
) -> Result<ToolDescriptor, String> {
    let id = read_abi_utf8(descriptor.id, "tool id")?;
    let label = read_abi_utf8(descriptor.label, "tool label")?;
    let icon_path = read_abi_utf8(descriptor.icon_path, "tool icon path")?;
    let silhouette_path = read_abi_utf8(descriptor.silhouette_path, "tool silhouette path")?;
    Ok(ToolDescriptor {
        id: ContentId::parse(&id)?,
        label,
        icon_path,
        silhouette_path: (!silhouette_path.trim().is_empty()).then_some(silhouette_path),
    })
}

unsafe fn overlay_descriptor_from_abi(
    plugin_id: &PluginId,
    descriptor: &FluxOverlayDescriptor,
) -> Result<RuntimeOverlayDescriptor, String> {
    let id = read_abi_utf8(descriptor.id, "overlay id")?;
    let label = read_abi_utf8(descriptor.label, "overlay label")?;
    let hotkey = read_abi_utf8(descriptor.hotkey, "overlay hotkey")?;
    let render_policy = match descriptor.render_policy {
        0 => OverlayRenderPolicy::CoreDefault,
        1 => OverlayRenderPolicy::PluginControlled,
        other => return Err(format!("unknown overlay render policy {}", other)),
    };
    Ok(RuntimeOverlayDescriptor {
        id: ContentId::parse(&id)?,
        plugin_id: plugin_id.clone(),
        label,
        hotkey: (!hotkey.trim().is_empty()).then_some(hotkey),
        render_policy,
    })
}

unsafe fn save_chunk_descriptor_from_abi(
    plugin_id: &PluginId,
    descriptor: &FluxSaveChunkDescriptor,
) -> Result<SaveChunkDescriptor, String> {
    let id = read_abi_utf8(descriptor.id, "save chunk id")?;
    Ok(SaveChunkDescriptor {
        id: ContentId::parse(&id)?,
        plugin_id: plugin_id.clone(),
        version: descriptor.version,
    })
}

unsafe fn gas_substance_from_abi(
    plugin_id: &PluginId,
    descriptor: &FluxGasSubstanceDescriptor,
) -> Result<SubstanceDefinition, String> {
    let id = read_abi_utf8(descriptor.id, "gas substance id")?;
    let label = read_abi_utf8(descriptor.label, "gas substance label")?;
    let alias = read_abi_utf8(descriptor.alias, "gas substance alias")?;
    let aliases = if alias.trim().is_empty() {
        Vec::new()
    } else {
        vec![alias]
    };
    SubstanceDefinition::gas(
        SubstanceId::parse(&id)?,
        plugin_id.clone(),
        label,
        descriptor.molecular_mass,
        [descriptor.color_r, descriptor.color_g, descriptor.color_b],
        aliases,
    )
}

unsafe fn read_abi_utf8(slice: FluxUtf8Slice, field_name: &str) -> Result<String, String> {
    if slice.len == 0 {
        return Ok(String::new());
    }
    if slice.ptr.is_null() {
        return Err(format!("{field_name} has null pointer and non-zero length"));
    }
    let bytes = std::slice::from_raw_parts(slice.ptr, slice.len);
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|error| format!("{field_name} is not valid UTF-8: {error}"))
}

fn cache_generation_root(
    plugin_id: &crate::plugins::id::PluginId,
) -> Result<PathBuf, PluginContractError> {
    let generation = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            PluginContractError::Io(format!(
                "failed to build plugin cache generation id: {}",
                error
            ))
        })?
        .as_nanos();
    let safe_id = plugin_id
        .as_str()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    Ok(std::env::temp_dir()
        .join("FluxEngine")
        .join("plugin_cache")
        .join(format!("{}_{}_{}", safe_id, std::process::id(), generation)))
}

unsafe fn load_symbol<T: Copy>(
    library: &Library,
    symbol_name: &[u8],
    display_name: &str,
) -> Result<T, PluginContractError> {
    let symbol = library.get::<T>(symbol_name).map_err(|error| {
        PluginContractError::Dll(format!(
            "DLL is missing required export '{}': {}",
            display_name, error
        ))
    })?;
    Ok(*symbol)
}

unsafe extern "C" fn write_host_log_message(
    context: *mut c_void,
    _level: u32,
    message: FluxUtf8Slice,
) -> FluxStatus {
    if context.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let target = &mut *(context.cast::<String>());
    if message.len == 0 {
        target.clear();
        return FluxStatus::OK;
    }
    if message.ptr.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let bytes = std::slice::from_raw_parts(message.ptr, message.len);
    match std::str::from_utf8(bytes) {
        Ok(text) => {
            target.clear();
            target.push_str(text);
            FluxStatus::OK
        }
        Err(_) => FluxStatus::INVALID_ARGUMENT,
    }
}

fn status_error(
    operation: &str,
    status: FluxStatus,
    host_error_message: &str,
    builder: fn(String) -> PluginContractError,
) -> PluginContractError {
    if !host_error_message.trim().is_empty() {
        return builder(format!(
            "{} failed with status {}: {}",
            operation,
            status.code(),
            host_error_message.trim()
        ));
    }

    builder(format!(
        "{} failed with status {}",
        operation,
        status.code()
    ))
}

fn path_to_utf8_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        fs::{self, File},
        io::Write,
        path::{Path, PathBuf},
        process::Command,
    };

    use libloading::Library;
    use zip::{write::SimpleFileOptions, ZipWriter};

    use super::{read_packaged_plugin_candidate, validate_runtime_registration};
    use crate::plugins::{
        abi::{FluxPluginDispatchFn, FLUX_PLUGIN_DISPATCH_EXPORT_NAME},
        api::events::PluginEvent,
        id::ENGINE_PLUGIN_API_VERSION_VALUE,
        PluginId, PluginManifest, PluginRuntimeRegistration, PluginSubscriptionRegistration,
        SubstanceDefinition, SubstanceId,
    };

    fn make_temp_archive_path(prefix: &str) -> PathBuf {
        let unique = format!(
            "{}_{}_{}.fluxplugin",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    fn cargo_binary() -> PathBuf {
        if let Ok(path) = env::var("CARGO") {
            let path = PathBuf::from(path);
            if path.exists() {
                return path;
            }
        }

        let fallback = PathBuf::from(r"C:\Users\andreybotanic\.cargo\bin\cargo.exe");
        if fallback.exists() {
            return fallback;
        }

        PathBuf::from("cargo")
    }

    fn demo_plugin_root(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("plugins")
            .join(name)
    }

    fn build_demo_plugin_cdylib(crate_name: &str, dll_name: &str) -> PathBuf {
        let crate_root = demo_plugin_root(crate_name);
        let status = Command::new(cargo_binary())
            .arg("build")
            .arg("--manifest-path")
            .arg(crate_root.join("Cargo.toml"))
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
            .status()
            .expect("spawn cargo build for demo plugin");
        assert!(status.success(), "demo plugin cargo build failed");

        crate_root.join("target").join("debug").join(dll_name)
    }

    fn sample_manifest(content: bool) -> PluginManifest {
        PluginManifest::from_str(&format!(
            r#"id = "sample.plugin"
display_name = "Sample"
version = "1.0.0"
api_version = {ENGINE_PLUGIN_API_VERSION_VALUE}
dll = "bin/sample.dll"
configs = "config"
assets = "assets"
content = {content}
"#
        ))
        .expect("manifest")
    }

    #[test]
    fn plugin_contract_archive_rejects_parent_escape() {
        let archive_path = make_temp_archive_path("flux_plugin_escape");
        let file = File::create(&archive_path).expect("create archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        writer
            .start_file("../manifest.toml", options)
            .expect("start escaped manifest");
        writer
            .write_all(
                format!(
                    r#"id = "escape.test"
display_name = "Escape"
version = "1.0.0"
api_version = {}
dll = "bin/test.dll"
configs = "config"
assets = "assets"
content = false
"#,
                    ENGINE_PLUGIN_API_VERSION_VALUE
                )
                .as_bytes(),
            )
            .expect("write manifest");
        writer.finish().expect("finish archive");

        let error = read_packaged_plugin_candidate(&archive_path).expect_err("must fail");
        assert!(error.to_string().contains("must not contain '..'"));

        let _ = fs::remove_file(archive_path);
    }

    #[test]
    fn plugin_contract_rejects_content_registration_from_non_content_plugin() {
        let manifest = sample_manifest(false);
        let registration = PluginRuntimeRegistration {
            gas_substances: vec![SubstanceDefinition::gas(
                SubstanceId::parse("sample.plugin.substance.neon").expect("substance id"),
                PluginId::parse("sample.plugin").expect("plugin id"),
                "Neon",
                20.180,
                [1.0, 0.32, 0.78],
                vec!["neon".to_string()],
            )
            .expect("substance")],
            ..Default::default()
        };

        let error = validate_runtime_registration(&manifest, &registration)
            .expect_err("non-content plugin must not register content");
        assert!(error.to_string().contains("content = false"));
    }

    #[test]
    fn plugin_contract_rejects_registered_substance_outside_plugin_namespace() {
        let manifest = sample_manifest(true);
        let registration = PluginRuntimeRegistration {
            gas_substances: vec![SubstanceDefinition::gas(
                SubstanceId::parse("other.plugin.substance.neon").expect("substance id"),
                PluginId::parse("sample.plugin").expect("plugin id"),
                "Neon",
                20.180,
                [1.0, 0.32, 0.78],
                vec!["neon".to_string()],
            )
            .expect("substance")],
            ..Default::default()
        };

        let error = validate_runtime_registration(&manifest, &registration)
            .expect_err("foreign namespace must fail");
        assert!(error.to_string().contains("outside plugin namespace"));
    }

    #[test]
    fn plugin_contract_accepts_event_subscription_registration() {
        let manifest = sample_manifest(false);
        let registration = PluginRuntimeRegistration {
            subscriptions: vec![PluginSubscriptionRegistration {
                event_kind: PluginEvent::KeyPressed,
            }],
            ..Default::default()
        };

        validate_runtime_registration(&manifest, &registration).expect("registration must pass");
    }

    #[test]
    fn plugin_contract_rejects_duplicate_event_subscription_registration() {
        let manifest = sample_manifest(false);
        let registration = PluginRuntimeRegistration {
            subscriptions: vec![
                PluginSubscriptionRegistration {
                    event_kind: PluginEvent::KeyPressed,
                },
                PluginSubscriptionRegistration {
                    event_kind: PluginEvent::KeyPressed,
                },
            ],
            ..Default::default()
        };

        let error = validate_runtime_registration(&manifest, &registration)
            .expect_err("duplicate event kind must fail");
        assert!(error.to_string().contains("duplicate event subscription"));
    }

    #[test]
    fn plugin_contract_requires_unified_dispatch_export() {
        let dll_path =
            build_demo_plugin_cdylib("flux_api_tick_demo_plugin", "flux_api_tick_demo_plugin.dll");
        let library = unsafe { Library::new(dll_path) }.expect("load demo plugin dll");
        unsafe {
            library
                .get::<FluxPluginDispatchFn>(FLUX_PLUGIN_DISPATCH_EXPORT_NAME)
                .expect("dispatch export must exist");
            assert!(
                library
                    .get::<unsafe extern "C" fn()>(b"onSimulationPreCellGasStep\0")
                    .is_err(),
                "plugin must not expose named v4 event handlers anymore"
            );
        }
    }
}

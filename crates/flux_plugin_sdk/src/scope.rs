use std::{cell::RefCell, ffi::c_void, path::PathBuf, ptr};

use bevy_math::Vec2;
use flux_plugin_abi::{FluxHostApi, FluxRuntimeHost, FluxUtf8Slice, FluxWriteLogFn};

use crate::{ContentId, InputModifiers, OverlayModeId, PluginError};

#[derive(Clone, Copy)]
pub(crate) struct InitLogScope {
    pub callback: Option<FluxWriteLogFn>,
    pub context: *mut c_void,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DispatchState {
    pub modifiers: InputModifiers,
    pub active_tool_id: Option<ContentId>,
    pub cursor_world: Option<Vec2>,
    pub cursor_screen: Option<Vec2>,
    pub is_pointer_over_ui: bool,
    pub requested_overlay: Option<OverlayModeId>,
    pub active_overlay: Option<OverlayModeId>,
}

#[derive(Clone, Copy)]
struct Scope {
    host: *mut FluxRuntimeHost,
    init_log: Option<InitLogScope>,
}

thread_local! {
    static CURRENT_SCOPE: RefCell<Scope> = const { RefCell::new(Scope {
        host: ptr::null_mut(),
        init_log: None,
    }) };
    static CURRENT_DISPATCH_STATE: RefCell<DispatchState> = RefCell::new(DispatchState::default());
}

pub(crate) fn with_runtime_host<T>(
    api_name: &'static str,
    f: impl FnOnce(&FluxRuntimeHost) -> Result<T, PluginError>,
) -> Result<T, PluginError> {
    CURRENT_SCOPE.with(|scope| {
        let scope = scope.borrow();
        let host = unsafe { scope.host.as_ref() }
            .ok_or(PluginError::ApiUnavailable(api_name))?;
        f(host)
    })
}

pub(crate) fn with_dispatch_state<T>(f: impl FnOnce(&DispatchState) -> T) -> T {
    CURRENT_DISPATCH_STATE.with(|state| f(&state.borrow()))
}

pub(crate) fn set_dispatch_state(next: DispatchState) {
    CURRENT_DISPATCH_STATE.with(|state| *state.borrow_mut() = next);
}

pub(crate) fn with_init_scope<T>(host: &FluxHostApi, f: impl FnOnce() -> T) -> T {
    CURRENT_SCOPE.with(|scope| {
        let previous = *scope.borrow();
        *scope.borrow_mut() = Scope {
            host: ptr::null_mut(),
            init_log: Some(InitLogScope {
                callback: host.write_log_fn,
                context: host.log_context,
            }),
        };
        let value = f();
        *scope.borrow_mut() = previous;
        value
    })
}

pub(crate) fn with_runtime_scope<T>(
    host: *mut FluxRuntimeHost,
    state: DispatchState,
    f: impl FnOnce() -> T,
) -> T {
    CURRENT_SCOPE.with(|scope| {
        let previous = *scope.borrow();
        *scope.borrow_mut() = Scope {
            host,
            init_log: None,
        };
        set_dispatch_state(state);
        let value = f();
        *scope.borrow_mut() = previous;
        set_dispatch_state(DispatchState::default());
        value
    })
}

pub(crate) fn write_log(level: u32, message: &str) -> Result<(), PluginError> {
    let runtime_result = with_runtime_host("logger", |host| {
        let Some(callback) = host.write_log_fn else {
            return Err(PluginError::Unsupported("logger"));
        };
        unsafe { callback(host.context, level, FluxUtf8Slice::from_str(message)) }
            .into_result()
            .map_err(|_| PluginError::message("host log callback failed"))
    });
    if runtime_result.is_ok() {
        return runtime_result;
    }

    CURRENT_SCOPE.with(|scope| {
        let scope = scope.borrow();
        let init_log = scope.init_log.ok_or(PluginError::ApiUnavailable("logger"))?;
        let callback = init_log
            .callback
            .ok_or(PluginError::Unsupported("logger"))?;
        unsafe { callback(init_log.context, level, FluxUtf8Slice::from_str(message)) }
            .into_result()
            .map_err(|_| PluginError::message("host log callback failed"))
    })
}

pub(crate) fn parse_utf8_path(slice: FluxUtf8Slice, label: &str) -> Result<PathBuf, PluginError> {
    let text = parse_utf8(slice, label)?;
    Ok(PathBuf::from(text))
}

pub(crate) fn parse_utf8(slice: FluxUtf8Slice, label: &str) -> Result<String, PluginError> {
    if slice.len == 0 {
        return Ok(String::new());
    }
    if slice.ptr.is_null() {
        return Err(PluginError::InvalidArgument(format!("{label} pointer is null")));
    }
    let bytes = unsafe { std::slice::from_raw_parts(slice.ptr, slice.len) };
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|error| PluginError::InvalidArgument(format!("{label} is not valid UTF-8: {error}")))
}

use std::marker::PhantomData;

use flux_plugin_abi::{
    FluxEntityDescriptor, FluxGasSubstanceDescriptor, FluxOverlayDescriptor,
    FluxPanelDescriptor, FluxRegistrar, FluxSaveChunkDescriptor, FluxStatus,
    FluxSubscriptionDescriptor, FluxToolDescriptor, FluxUtf8Slice,
};

use crate::{
    AbiEventPayload, EntityDescriptor, OverlayDescriptor, OverlayRenderPolicy, PanelDescriptor,
    PluginError, PluginEvent, SaveChunkDescriptor, SubstanceDescriptor, ToolDescriptor,
};

/// One typed plugin event handler.
pub type Handler<P, E> = fn(&mut P, &E) -> Result<(), PluginError>;

pub(crate) struct RegisteredHandler<P> {
    pub event_kind: PluginEvent,
    pub handler: *const (),
    pub dispatch: unsafe fn(&mut P, *const (), *const u8, usize) -> Result<(), PluginError>,
}

/// Registration helper used by `Plugin::register`.
pub struct Registrar<'a, P> {
    plugin_id: crate::PluginId,
    raw: &'a mut FluxRegistrar,
    handlers: Vec<RegisteredHandler<P>>,
    registered_ids: Vec<crate::ContentId>,
    _marker: PhantomData<P>,
}

impl<'a, P> Registrar<'a, P> {
    pub(crate) fn new(plugin_id: crate::PluginId, raw: &'a mut FluxRegistrar) -> Self {
        Self {
            plugin_id,
            raw,
            handlers: Vec::new(),
            registered_ids: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Returns the current plugin id.
    pub fn plugin_id(&self) -> &crate::PluginId {
        &self.plugin_id
    }

    /// Returns `true` when one event kind is already subscribed.
    pub fn has_subscription(&self, event: PluginEvent) -> bool {
        self.handlers.iter().any(|registered| registered.event_kind == event)
    }

    /// Returns `true` when one content id has already been registered.
    pub fn has_registered_content(&self, id: &crate::ContentId) -> bool {
        self.registered_ids.iter().any(|registered| registered == id)
    }

    /// Registers one gas substance.
    pub fn register_substance(&mut self, descriptor: SubstanceDescriptor) -> Result<(), PluginError> {
        let callback = self
            .raw
            .register_gas_substance_fn
            .ok_or(PluginError::Unsupported("registrar.register_substance"))?;
        let abi = FluxGasSubstanceDescriptor {
            id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
            label: FluxUtf8Slice::from_str(&descriptor.label),
            alias: FluxUtf8Slice::from_str(&descriptor.alias),
            molecular_mass: descriptor.molecular_mass,
            color_r: descriptor.color[0],
            color_g: descriptor.color[1],
            color_b: descriptor.color[2],
        };
        unsafe { callback(self.raw.registration_context, &abi) }
            .into_result()
            .map_err(status_error)
    }

    /// Registers one entity descriptor.
    pub fn register_entity(&mut self, descriptor: EntityDescriptor) -> Result<(), PluginError> {
        let callback = self
            .raw
            .register_entity_fn
            .ok_or(PluginError::Unsupported("registrar.register_entity"))?;
        let abi = FluxEntityDescriptor {
            id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
            label: FluxUtf8Slice::from_str(&descriptor.label),
            icon_path: FluxUtf8Slice::from_str(&descriptor.icon_path),
            silhouette_path: FluxUtf8Slice::from_str(descriptor.silhouette_path.as_deref().unwrap_or("")),
        };
        unsafe { callback(self.raw.registration_context, &abi) }
            .into_result()
            .map_err(status_error)?;
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Registers one overlay descriptor.
    pub fn register_overlay(&mut self, descriptor: OverlayDescriptor) -> Result<(), PluginError> {
        let callback = self
            .raw
            .register_overlay_fn
            .ok_or(PluginError::Unsupported("registrar.register_overlay"))?;
        let abi = FluxOverlayDescriptor {
            id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
            label: FluxUtf8Slice::from_str(&descriptor.label),
            hotkey: FluxUtf8Slice::from_str(descriptor.hotkey.as_deref().unwrap_or("")),
            render_policy: match descriptor.render_policy {
                OverlayRenderPolicy::CoreDefault => 0,
                OverlayRenderPolicy::PluginControlled => 1,
            },
        };
        unsafe { callback(self.raw.registration_context, &abi) }
            .into_result()
            .map_err(status_error)?;
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Registers one tool descriptor.
    pub fn register_tool(&mut self, descriptor: ToolDescriptor) -> Result<(), PluginError> {
        let callback = self
            .raw
            .register_tool_fn
            .ok_or(PluginError::Unsupported("registrar.register_tool"))?;
        let abi = FluxToolDescriptor {
            id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
            label: FluxUtf8Slice::from_str(&descriptor.label),
            icon_path: FluxUtf8Slice::from_str(&descriptor.icon_path),
            silhouette_path: FluxUtf8Slice::from_str(descriptor.silhouette_path.as_deref().unwrap_or("")),
        };
        unsafe { callback(self.raw.registration_context, &abi) }
            .into_result()
            .map_err(status_error)?;
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Registers one panel descriptor.
    pub fn register_panel(&mut self, descriptor: PanelDescriptor) -> Result<(), PluginError> {
        let callback = self
            .raw
            .register_panel_fn
            .ok_or(PluginError::Unsupported("registrar.register_panel"))?;
        let abi = FluxPanelDescriptor {
            id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
            title: FluxUtf8Slice::from_str(&descriptor.title),
        };
        unsafe { callback(self.raw.registration_context, &abi) }
            .into_result()
            .map_err(status_error)?;
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Registers one save chunk descriptor.
    pub fn register_save_chunk(&mut self, descriptor: SaveChunkDescriptor) -> Result<(), PluginError> {
        let callback = self
            .raw
            .register_save_chunk_fn
            .ok_or(PluginError::Unsupported("registrar.register_save_chunk"))?;
        let abi = FluxSaveChunkDescriptor {
            id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
            version: descriptor.version,
        };
        unsafe { callback(self.raw.registration_context, &abi) }
            .into_result()
            .map_err(status_error)?;
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Subscribes one typed handler to one event.
    pub fn subscribe<E>(&mut self, event: PluginEvent, handler: Handler<P, E>) -> Result<(), PluginError>
    where
        E: AbiEventPayload,
    {
        if E::KIND != event
            && !matches!(
                (E::KIND, event),
                (PluginEvent::MouseDownCell, PluginEvent::MouseMoveCell)
                    | (PluginEvent::MouseDownCell, PluginEvent::MouseUpCell)
                    | (PluginEvent::MouseDownCell, PluginEvent::MouseEnterCell)
                    | (PluginEvent::MouseDownCell, PluginEvent::MouseLeaveCell)
                    | (PluginEvent::KeyPressed, PluginEvent::KeyReleased)
                    | (PluginEvent::EntityPlaced, PluginEvent::EntityRemoved)
            )
        {
            return Err(PluginError::message(format!(
                "handler payload {:?} does not match subscription {:?}",
                E::KIND, event
            )));
        }
        if self.has_subscription(event) {
            return Err(PluginError::message(format!(
                "plugin already subscribed to {:?}",
                event
            )));
        }
        let callback = self
            .raw
            .register_subscription_fn
            .ok_or(PluginError::Unsupported("registrar.subscribe"))?;
        let abi = FluxSubscriptionDescriptor {
            event_kind: event.abi_kind() as u32,
        };
        unsafe { callback(self.raw.registration_context, &abi) }
            .into_result()
            .map_err(status_error)?;
        self.handlers.push(RegisteredHandler {
            event_kind: event,
            handler: handler as *const (),
            dispatch: dispatch_handler::<P, E>,
        });
        Ok(())
    }

    pub(crate) fn finish(self) -> Vec<RegisteredHandler<P>> {
        self.handlers
    }
}

unsafe fn dispatch_handler<P, E>(
    plugin: &mut P,
    handler: *const (),
    payload: *const u8,
    payload_len: usize,
) -> Result<(), PluginError>
where
    E: AbiEventPayload,
{
    let handler: Handler<P, E> = std::mem::transmute(handler);
    let event = E::decode(payload, payload_len)?;
    handler(plugin, &event)
}

fn status_error(status: FluxStatus) -> PluginError {
    match status {
        FluxStatus::INVALID_ARGUMENT => PluginError::InvalidArgument("registrar rejected input".to_string()),
        FluxStatus::API_UNAVAILABLE => PluginError::ApiUnavailable("registrar"),
        FluxStatus::UNSUPPORTED => PluginError::Unsupported("registrar"),
        _ => PluginError::message(format!("registrar call failed with status {}", status.code())),
    }
}

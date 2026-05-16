use std::marker::PhantomData;

use flux_plugin_abi::{
    FluxEntityCategoryDescriptor, FluxEntityDescriptor, FluxEntityStateDescriptor,
    FluxGasSubstanceDescriptor, FluxOverlayDescriptor, FluxOverlayMaterialDescriptor,
    FluxRegistrar, FluxSaveChunkDescriptor, FluxStatus, FluxSubscriptionDescriptor,
    FluxToolDescriptor, FluxUtf8Slice, FLUX_REGISTRATION_PHASE_CATEGORIES,
    FLUX_REGISTRATION_PHASE_CONTENT,
};

use crate::{
    AbiEventPayload, BuiltinEventPayload, EntityCategoryDescriptor, EntityCategoryRef,
    EntityDescriptor, EntitySpriteTransform, OverlayDescriptor, OverlayMaterialDescriptor,
    OverlayRenderPolicy, PluginError, PluginEvent, SaveChunkDescriptor, SubstanceDescriptor,
    ToolDescriptor,
};

/// One typed plugin event handler.
pub type Handler<P, E> = fn(&mut P, &E) -> Result<(), PluginError>;

pub(crate) struct RegisteredHandler<P> {
    pub event_kind: PluginEvent,
    pub handler: *const (),
    pub abi_dispatch: unsafe fn(&mut P, *const (), *const u8, usize) -> Result<(), PluginError>,
    pub builtin_dispatch:
        unsafe fn(&mut P, *const (), &dyn BuiltinEventPayload) -> Result<(), PluginError>,
}

/// In-memory registration snapshot collected for built-in plugins.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemoryRegistration {
    pub substances: Vec<SubstanceDescriptor>,
    pub entity_categories: Vec<EntityCategoryDescriptor>,
    pub entities: Vec<EntityDescriptor>,
    pub overlays: Vec<OverlayDescriptor>,
    pub overlay_materials: Vec<OverlayMaterialDescriptor>,
    pub tools: Vec<ToolDescriptor>,
    pub save_chunks: Vec<SaveChunkDescriptor>,
    pub subscriptions: Vec<PluginEvent>,
}

enum RegistrarSink<'a> {
    Abi(&'a mut FluxRegistrar),
    Memory(&'a mut MemoryRegistration),
}

/// Active registration phase requested by the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationPhase {
    Categories,
    Content,
}

/// Registration helper used by `Plugin::register`.
pub struct Registrar<'a, P> {
    plugin_id: crate::PluginId,
    phase: RegistrationPhase,
    sink: RegistrarSink<'a>,
    handlers: Vec<RegisteredHandler<P>>,
    registered_ids: Vec<crate::ContentId>,
    known_entity_categories: Vec<EntityCategoryRef>,
    _marker: PhantomData<P>,
}

impl<'a, P> Registrar<'a, P> {
    pub(crate) fn new(plugin_id: crate::PluginId, raw: &'a mut FluxRegistrar) -> Self {
        Self {
            plugin_id,
            phase: match raw.registration_phase {
                FLUX_REGISTRATION_PHASE_CATEGORIES => RegistrationPhase::Categories,
                FLUX_REGISTRATION_PHASE_CONTENT => RegistrationPhase::Content,
                _ => RegistrationPhase::Content,
            },
            sink: RegistrarSink::Abi(raw),
            handlers: Vec::new(),
            registered_ids: Vec::new(),
            known_entity_categories: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn new_memory(
        plugin_id: crate::PluginId,
        memory: &'a mut MemoryRegistration,
        phase: RegistrationPhase,
    ) -> Self {
        let known_entity_categories = memory
            .entity_categories
            .iter()
            .map(|descriptor| EntityCategoryRef::new(descriptor.id.clone()))
            .collect::<Vec<_>>();
        Self {
            plugin_id,
            phase,
            sink: RegistrarSink::Memory(memory),
            handlers: Vec::new(),
            registered_ids: Vec::new(),
            known_entity_categories,
            _marker: PhantomData,
        }
    }

    pub(crate) fn seed_entity_categories(&mut self, categories: &[EntityCategoryRef]) {
        for category in categories {
            if self
                .known_entity_categories
                .iter()
                .any(|known| known.id() == category.id())
            {
                continue;
            }
            self.known_entity_categories.push(category.clone());
        }
    }

    /// Returns the current plugin id.
    pub fn plugin_id(&self) -> &crate::PluginId {
        &self.plugin_id
    }

    /// Returns the registration phase requested by the host.
    pub fn phase(&self) -> RegistrationPhase {
        self.phase
    }

    /// Returns `true` when the registrar is in category-registration phase.
    pub fn is_category_phase(&self) -> bool {
        self.phase == RegistrationPhase::Categories
    }

    /// Returns `true` when the registrar is in content-registration phase.
    pub fn is_content_phase(&self) -> bool {
        self.phase == RegistrationPhase::Content
    }

    /// Returns `true` when one event kind is already subscribed.
    pub fn has_subscription(&self, event: PluginEvent) -> bool {
        self.handlers
            .iter()
            .any(|registered| registered.event_kind == event)
    }

    /// Returns `true` when one content id has already been registered.
    pub fn has_registered_content(&self, id: &crate::ContentId) -> bool {
        self.registered_ids
            .iter()
            .any(|registered| registered == id)
    }

    /// Registers one entity category.
    pub fn register_entity_category(
        &mut self,
        descriptor: EntityCategoryDescriptor,
    ) -> Result<EntityCategoryRef, PluginError> {
        match &mut self.sink {
            RegistrarSink::Abi(raw) => {
                let callback = raw
                    .register_entity_category_fn
                    .ok_or(PluginError::Unsupported(
                        "registrar.register_entity_category",
                    ))?;
                let abi = FluxEntityCategoryDescriptor {
                    id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
                    label: FluxUtf8Slice::from_str(&descriptor.label),
                    icon_path: FluxUtf8Slice::from_str(&descriptor.icon_path),
                };
                unsafe { callback(raw.registration_context, &abi) }
                    .into_result()
                    .map_err(status_error)?;
            }
            RegistrarSink::Memory(memory) => memory.entity_categories.push(descriptor.clone()),
        }
        let category_ref = EntityCategoryRef::new(descriptor.id.clone());
        self.known_entity_categories.push(category_ref.clone());
        self.registered_ids.push(descriptor.id);
        Ok(category_ref)
    }

    /// Returns one previously registered entity category.
    pub fn entity_category(&self, id: &crate::ContentId) -> Option<EntityCategoryRef> {
        self.known_entity_categories
            .iter()
            .find(|category| category.id() == id)
            .cloned()
    }

    /// Returns all known entity categories visible in this registrar scope.
    pub fn entity_categories(&self) -> Vec<EntityCategoryRef> {
        self.known_entity_categories.clone()
    }

    /// Registers one gas substance.
    pub fn register_substance(
        &mut self,
        descriptor: SubstanceDescriptor,
    ) -> Result<(), PluginError> {
        match &mut self.sink {
            RegistrarSink::Abi(raw) => {
                let callback = raw
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
                unsafe { callback(raw.registration_context, &abi) }
                    .into_result()
                    .map_err(status_error)
            }
            RegistrarSink::Memory(memory) => {
                memory.substances.push(descriptor);
                Ok(())
            }
        }
    }

    /// Registers one entity descriptor.
    pub fn register_entity(&mut self, descriptor: EntityDescriptor) -> Result<(), PluginError> {
        if let Some(category) = descriptor.category.as_ref() {
            let category_id = category.id();
            if self.entity_category(category_id).is_none() {
                return Err(PluginError::message(format!(
                    "entity '{}' references unknown category '{}'; register category first and resolve it via registrar.entity_category(...)",
                    descriptor.id,
                    category_id
                )));
            }
        }
        match &mut self.sink {
            RegistrarSink::Abi(raw) => {
                let callback = raw
                    .register_entity_fn
                    .ok_or(PluginError::Unsupported("registrar.register_entity"))?;
                let abi_states = descriptor
                    .states
                    .iter()
                    .map(|state| FluxEntityStateDescriptor {
                        state: state.state.value(),
                        sprite_path: FluxUtf8Slice::from_str(&state.sprite_path),
                        transform: encode_entity_sprite_transform(state.transform),
                    })
                    .collect::<Vec<_>>();
                let abi = FluxEntityDescriptor {
                    id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
                    label: FluxUtf8Slice::from_str(&descriptor.label),
                    icon_path: FluxUtf8Slice::from_str(&descriptor.icon_path),
                    silhouette_path: FluxUtf8Slice::from_str(
                        descriptor.silhouette_path.as_deref().unwrap_or(""),
                    ),
                    category_id: FluxUtf8Slice::from_str(
                        descriptor
                            .category
                            .as_ref()
                            .map(|category| category.id().as_str())
                            .unwrap_or(""),
                    ),
                    default_state: descriptor.default_state.value(),
                    states: abi_states.as_ptr(),
                    states_len: abi_states.len(),
                };
                unsafe { callback(raw.registration_context, &abi) }
                    .into_result()
                    .map_err(status_error)?;
            }
            RegistrarSink::Memory(memory) => memory.entities.push(descriptor.clone()),
        }
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Registers one overlay descriptor.
    pub fn register_overlay(&mut self, descriptor: OverlayDescriptor) -> Result<(), PluginError> {
        if let Some(graph) = &descriptor.graph {
            graph
                .validate()
                .map_err(|error| PluginError::InvalidArgument(error.to_string()))?;
        }
        match &mut self.sink {
            RegistrarSink::Abi(raw) => {
                let callback = raw
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
                unsafe { callback(raw.registration_context, &abi) }
                    .into_result()
                    .map_err(status_error)?;
            }
            RegistrarSink::Memory(memory) => memory.overlays.push(descriptor.clone()),
        }
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Registers one plugin-owned overlay material descriptor.
    pub fn register_overlay_material(
        &mut self,
        descriptor: OverlayMaterialDescriptor,
    ) -> Result<(), PluginError> {
        match &mut self.sink {
            RegistrarSink::Abi(raw) => {
                let callback = raw
                    .register_overlay_material_fn
                    .ok_or(PluginError::Unsupported(
                        "registrar.register_overlay_material",
                    ))?;
                let abi = FluxOverlayMaterialDescriptor {
                    id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
                    label: FluxUtf8Slice::from_str(&descriptor.label),
                    shader_path: FluxUtf8Slice::from_str(&descriptor.shader_path),
                };
                unsafe { callback(raw.registration_context, &abi) }
                    .into_result()
                    .map_err(status_error)?;
            }
            RegistrarSink::Memory(memory) => memory.overlay_materials.push(descriptor.clone()),
        }
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Registers one tool descriptor.
    pub fn register_tool(&mut self, descriptor: ToolDescriptor) -> Result<(), PluginError> {
        match &mut self.sink {
            RegistrarSink::Abi(raw) => {
                let callback = raw
                    .register_tool_fn
                    .ok_or(PluginError::Unsupported("registrar.register_tool"))?;
                let abi = FluxToolDescriptor {
                    id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
                    label: FluxUtf8Slice::from_str(&descriptor.label),
                    icon_path: FluxUtf8Slice::from_str(&descriptor.icon_path),
                    silhouette_path: FluxUtf8Slice::from_str(
                        descriptor.silhouette_path.as_deref().unwrap_or(""),
                    ),
                };
                unsafe { callback(raw.registration_context, &abi) }
                    .into_result()
                    .map_err(status_error)?;
            }
            RegistrarSink::Memory(memory) => memory.tools.push(descriptor.clone()),
        }
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Registers one save chunk descriptor.
    pub fn register_save_chunk(
        &mut self,
        descriptor: SaveChunkDescriptor,
    ) -> Result<(), PluginError> {
        match &mut self.sink {
            RegistrarSink::Abi(raw) => {
                let callback = raw
                    .register_save_chunk_fn
                    .ok_or(PluginError::Unsupported("registrar.register_save_chunk"))?;
                let abi = FluxSaveChunkDescriptor {
                    id: FluxUtf8Slice::from_str(descriptor.id.as_str()),
                    version: descriptor.version,
                };
                unsafe { callback(raw.registration_context, &abi) }
                    .into_result()
                    .map_err(status_error)?;
            }
            RegistrarSink::Memory(memory) => memory.save_chunks.push(descriptor.clone()),
        }
        self.registered_ids.push(descriptor.id);
        Ok(())
    }

    /// Subscribes one typed handler to one event.
    pub fn subscribe<E>(
        &mut self,
        event: PluginEvent,
        handler: Handler<P, E>,
    ) -> Result<(), PluginError>
    where
        E: AbiEventPayload + BuiltinEventPayload,
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
                E::KIND,
                event
            )));
        }
        if self.has_subscription(event) {
            return Err(PluginError::message(format!(
                "plugin already subscribed to {:?}",
                event
            )));
        }
        match &mut self.sink {
            RegistrarSink::Abi(raw) => {
                let callback = raw
                    .register_subscription_fn
                    .ok_or(PluginError::Unsupported("registrar.subscribe"))?;
                let abi = FluxSubscriptionDescriptor {
                    event_kind: event.abi_kind() as u32,
                };
                unsafe { callback(raw.registration_context, &abi) }
                    .into_result()
                    .map_err(status_error)?;
            }
            RegistrarSink::Memory(memory) => memory.subscriptions.push(event),
        }
        self.handlers.push(RegisteredHandler {
            event_kind: event,
            handler: handler as *const (),
            abi_dispatch: dispatch_abi_handler::<P, E>,
            builtin_dispatch: dispatch_builtin_handler::<P, E>,
        });
        Ok(())
    }

    pub(crate) fn finish(self) -> Vec<RegisteredHandler<P>> {
        self.handlers
    }
}

unsafe fn dispatch_abi_handler<P, E>(
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

unsafe fn dispatch_builtin_handler<P, E>(
    plugin: &mut P,
    handler: *const (),
    payload: &dyn BuiltinEventPayload,
) -> Result<(), PluginError>
where
    E: BuiltinEventPayload,
{
    let handler: Handler<P, E> = std::mem::transmute(handler);
    let event = payload
        .as_any()
        .downcast_ref::<E>()
        .ok_or_else(|| PluginError::message("builtin event payload type mismatch"))?;
    handler(plugin, event)
}

fn status_error(status: FluxStatus) -> PluginError {
    match status {
        FluxStatus::INVALID_ARGUMENT => {
            PluginError::InvalidArgument("registrar rejected input".to_string())
        }
        FluxStatus::API_UNAVAILABLE => PluginError::ApiUnavailable("registrar"),
        FluxStatus::UNSUPPORTED => PluginError::Unsupported("registrar"),
        _ => PluginError::message(format!(
            "registrar call failed with status {}",
            status.code()
        )),
    }
}

fn encode_entity_sprite_transform(transform: EntitySpriteTransform) -> u32 {
    match transform {
        EntitySpriteTransform::None => 0,
        EntitySpriteTransform::Rot90 => 1,
        EntitySpriteTransform::Rot180 => 2,
        EntitySpriteTransform::Rot270 => 3,
        EntitySpriteTransform::FlipX => 4,
        EntitySpriteTransform::FlipY => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryRegistration, Registrar, RegistrationPhase};
    use crate::{
        ContentId, EntityCategoryDescriptor, EntityCategoryRef, EntityDescriptor,
        EntitySpriteTransform, EntityStateDescriptor, PackedState, PluginId,
    };

    #[test]
    fn registrar_content_phase_can_lookup_categories_from_previous_phase() {
        let plugin_id = PluginId::parse("sample.plugin").expect("plugin id");
        let category_id = ContentId::parse("sample.plugin.category.cells").expect("category id");
        let mut memory = MemoryRegistration::default();
        {
            let mut categories = Registrar::<()>::new_memory(
                plugin_id.clone(),
                &mut memory,
                RegistrationPhase::Categories,
            );
            categories
                .register_entity_category(EntityCategoryDescriptor {
                    id: category_id.clone(),
                    label: "Cells".to_string(),
                    icon_path: "sprites/ui/tool_build.ktx2".to_string(),
                })
                .expect("register category");
        }

        let content =
            Registrar::<()>::new_memory(plugin_id, &mut memory, RegistrationPhase::Content);
        assert!(
            content.entity_category(&category_id).is_some(),
            "content phase must resolve typed category handles registered in categories phase"
        );
    }

    #[test]
    fn registrar_memory_registration_keeps_typed_entity_category_reference() {
        let plugin_id = PluginId::parse("sample.plugin").expect("plugin id");
        let category_id = ContentId::parse("sample.plugin.category.cells").expect("category id");
        let entity_id = ContentId::parse("sample.plugin.entity.widget").expect("entity id");
        let mut memory = MemoryRegistration::default();
        let category_ref = {
            let mut categories = Registrar::<()>::new_memory(
                plugin_id.clone(),
                &mut memory,
                RegistrationPhase::Categories,
            );
            categories
                .register_entity_category(EntityCategoryDescriptor {
                    id: category_id.clone(),
                    label: "Cells".to_string(),
                    icon_path: "sprites/ui/tool_build.ktx2".to_string(),
                })
                .expect("register category")
        };

        {
            let mut content =
                Registrar::<()>::new_memory(plugin_id, &mut memory, RegistrationPhase::Content);
            content
                .register_entity(EntityDescriptor {
                    id: entity_id,
                    label: "Widget".to_string(),
                    icon_path: "sprites/ui/tool_build.ktx2".to_string(),
                    silhouette_path: None,
                    default_state: PackedState(0),
                    states: vec![EntityStateDescriptor {
                        state: PackedState(0),
                        sprite_path: "sprites/world/widget_state_0.ktx2".to_string(),
                        transform: EntitySpriteTransform::None,
                    }],
                    category: Some(category_ref.clone()),
                    tags: Vec::new(),
                })
                .expect("register entity");
        }

        let registered = memory
            .entities
            .first()
            .expect("entity must be stored in memory registrar");
        assert_eq!(
            registered
                .category
                .as_ref()
                .map(|category| category.id().as_str()),
            Some(category_ref.id().as_str()),
            "entity payload must preserve typed category in registration snapshot"
        );
    }

    #[test]
    fn registrar_rejects_entity_with_unknown_typed_category_reference() {
        let plugin_id = PluginId::parse("sample.plugin").expect("plugin id");
        let unknown_category =
            ContentId::parse("sample.plugin.category.unknown").expect("unknown category id");
        let entity_id = ContentId::parse("sample.plugin.entity.widget").expect("entity id");
        let unknown_ref = EntityCategoryRef::new(unknown_category.clone());
        let mut memory = MemoryRegistration::default();
        let mut content =
            Registrar::<()>::new_memory(plugin_id, &mut memory, RegistrationPhase::Content);

        let error = content
            .register_entity(EntityDescriptor {
                id: entity_id,
                label: "Widget".to_string(),
                icon_path: "sprites/ui/tool_build.ktx2".to_string(),
                silhouette_path: None,
                default_state: PackedState(0),
                states: vec![EntityStateDescriptor {
                    state: PackedState(0),
                    sprite_path: "sprites/world/widget_state_0.ktx2".to_string(),
                    transform: EntitySpriteTransform::None,
                }],
                category: Some(unknown_ref),
                tags: Vec::new(),
            })
            .expect_err("unknown category must be rejected");
        assert!(
            error.to_string().contains("references unknown category"),
            "error must mention unknown category: {error}"
        );
    }
}

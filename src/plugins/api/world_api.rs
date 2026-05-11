use bevy::prelude::*;

use crate::{
    config::GasRegistry,
    plugins::{ContentRegistry, SubstanceId},
    render::OverlayMode,
    simulation::{gas::GasField, GpuRuntimeState},
    world::{
        grid::{
            is_boundary, is_editable_cell, CellKind, CellMaterial, WorldGrid, WORLD_HEIGHT,
            WORLD_WIDTH,
        },
        structures::{
            LayerKind, PlacedStructureId, PlacedStructureMap, StructureKind, StructureParams,
            StructureRotation,
        },
    },
};

use crate::world::structures::{PlacedStructure, StructureDescriptor};

/// Describes one gas amount inside a mixture using a stable substance id.
///
/// # Fields
/// - `substance_id`: Stable substance identifier for the gas species stored in this entry.
/// - `amount`: Rounded particle count for this species inside the queried cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GasAmount {
    pub substance_id: SubstanceId,
    pub amount: u32,
}

/// Describes gas content and cell-level gas velocity for one world cell.
///
/// # Fields
/// - `species`: Individual gas entries present in the cell, filtered to non-zero amounts.
/// - `total_amount`: Sum of all rounded gas particles currently stored in the cell.
/// - `velocity`: Cell-level gas velocity vector returned by the simulation state.
#[derive(Clone, Debug, PartialEq)]
pub struct GasMixture {
    pub species: Vec<GasAmount>,
    pub total_amount: u32,
    pub velocity: Vec2,
}

/// Read-only snapshot of one placed structure.
///
/// # Fields
/// - `id`: Stable runtime identifier of the placed structure instance.
/// - `kind`: Registered structure kind used to resolve content metadata.
/// - `origin`: Placement origin in world-cell coordinates.
/// - `rotation`: Canonical rotation stored for this structure instance.
/// - `params`: Editable runtime parameters attached to this structure.
/// - `occupied_cells`: All world cells currently occupied by the structure footprint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureInfo {
    pub id: PlacedStructureId,
    pub kind: StructureKind,
    pub origin: UVec2,
    pub rotation: StructureRotation,
    pub params: StructureParams,
    pub occupied_cells: Vec<UVec2>,
}

/// Read-only snapshot of one world cell.
///
/// # Fields
/// - `cell`: Queried world-cell coordinates.
/// - `in_bounds`: Always `true` for successful calls; included for explicit API consumers.
/// - `is_boundary`: Whether the cell belongs to the non-editable world border.
/// - `is_editable`: Whether plugin tools may mutate this cell through world APIs.
/// - `material`: Solid material occupying the cell, if the cell is not empty.
/// - `gas`: Complete gas mixture snapshot for the cell.
/// - `structures`: All placed structures that currently occupy the cell.
#[derive(Clone, Debug, PartialEq)]
pub struct CellInfo {
    pub cell: UVec2,
    pub in_bounds: bool,
    pub is_boundary: bool,
    pub is_editable: bool,
    pub material: Option<CellMaterial>,
    pub gas: GasMixture,
    pub structures: Vec<StructureInfo>,
}

/// Request used by plugins to place a registered structure.
///
/// # Fields
/// - `kind`: Registered structure kind to place.
/// - `origin`: Placement origin in world-cell coordinates.
/// - `rotation`: Rotation to use when resolving layer descriptors and footprint cells.
/// - `params`: Initial editable parameters stored on the created structure instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructurePlacement {
    pub kind: StructureKind,
    pub origin: UVec2,
    pub rotation: StructureRotation,
    pub params: StructureParams,
}

/// Error returned by world-facing plugin API operations.
///
/// # Variants
/// - `OutOfBounds`: The requested world cell lies outside the fixed simulation bounds.
/// - `NotEditable`: The target cell is inside the protected boundary and cannot be modified.
/// - `UnknownCellMaterial`: The provided cell material id is not registered in the content registry.
/// - `UnknownStructureKind`: The provided structure kind id is not registered in the content registry.
/// - `UnknownSubstance`: The provided substance id or alias is not registered in the gas registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldApiError {
    OutOfBounds(UVec2),
    NotEditable(UVec2),
    UnknownCellMaterial(String),
    UnknownStructureKind(String),
    UnknownSubstance(String),
}

/// Read-only world API exposed to plugin systems and event handlers.
///
/// # Fields
/// - `content_registry`: Read-only registry used to resolve cells, structures and overlays by stable ids.
/// - `gas_registry`: Read-only gas registry used to resolve substance ids and compact gas indices.
/// - `world`: Immutable access to the current world grid materials.
/// - `gas`: Immutable access to the current free-gas simulation field.
/// - `structures`: Immutable access to all placed structure instances in the world.
/// - `overlay_mode`: Overlay mode visible to the current caller, if the dispatch includes overlay context.
/// - `hovered_cell`: Hovered world cell visible to the current caller, if the dispatch includes input context.
pub struct WorldApi<'a> {
    pub content_registry: &'a ContentRegistry,
    pub gas_registry: &'a GasRegistry,
    pub world: &'a WorldGrid,
    pub gas: &'a GasField,
    pub structures: &'a PlacedStructureMap,
    pub overlay_mode: Option<OverlayMode>,
    pub hovered_cell: Option<UVec2>,
}

impl<'a> WorldApi<'a> {
    /// Returns the fixed world size in cells.
    ///
    pub fn get_world_size(&self) -> UVec2 {
        UVec2::new(WORLD_WIDTH, WORLD_HEIGHT)
    }

    /// Returns full read-only information for one cell.
    ///
    pub fn get_cell_info(&self, cell: UVec2) -> Result<CellInfo, WorldApiError> {
        self.ensure_in_bounds(cell)?;
        Ok(CellInfo {
            cell,
            in_bounds: true,
            is_boundary: is_boundary(cell.x, cell.y),
            is_editable: is_editable_cell(cell.x, cell.y),
            material: self.get_cell_material(cell)?,
            gas: self.get_cell_gas(cell)?,
            structures: self.get_cell_structures(cell)?,
        })
    }

    /// Returns the solid material occupying one cell, if any.
    ///
    pub fn get_cell_material(&self, cell: UVec2) -> Result<Option<CellMaterial>, WorldApiError> {
        self.ensure_in_bounds(cell)?;
        Ok(self.world.solid_material(cell.x, cell.y))
    }

    /// Returns the full gas mixture stored in one cell.
    ///
    pub fn get_cell_gas(&self, cell: UVec2) -> Result<GasMixture, WorldApiError> {
        self.ensure_in_bounds(cell)?;
        Ok(self.gas_mixture_from_counts(
            cell,
            (0..self.gas.gas_count())
                .map(|gas_index| self.gas.amount_rounded(cell.x, cell.y, gas_index))
                .collect(),
        ))
    }

    /// Returns one gas amount in a cell by stable substance id or alias.
    ///
    pub fn get_cell_gas_amount(&self, cell: UVec2, substance: &str) -> Result<u32, WorldApiError> {
        self.ensure_in_bounds(cell)?;
        let gas_index = self
            .gas_registry
            .index_of(substance)
            .ok_or_else(|| WorldApiError::UnknownSubstance(substance.to_string()))?;
        Ok(self.gas.amount_rounded(cell.x, cell.y, gas_index))
    }

    /// Returns the cell-level velocity for a gas if that gas is present in the cell.
    ///
    pub fn get_cell_gas_velocity(
        &self,
        cell: UVec2,
        substance: &str,
    ) -> Result<Vec2, WorldApiError> {
        let amount = self.get_cell_gas_amount(cell, substance)?;
        Ok(if amount > 0 {
            self.gas.velocity(cell.x, cell.y)
        } else {
            Vec2::ZERO
        })
    }

    /// Returns all placed structures occupying one cell.
    ///
    pub fn get_cell_structures(&self, cell: UVec2) -> Result<Vec<StructureInfo>, WorldApiError> {
        self.ensure_in_bounds(cell)?;
        Ok(self
            .structures
            .structures_at(cell.x, cell.y)
            .into_iter()
            .map(structure_info)
            .collect())
    }

    /// Returns structures that occupy one cell through a particular layer.
    ///
    pub fn get_cell_structures_on_layer(
        &self,
        cell: UVec2,
        layer: LayerKind,
    ) -> Result<Vec<StructureInfo>, WorldApiError> {
        self.ensure_in_bounds(cell)?;
        Ok(self
            .structures
            .structures_at(cell.x, cell.y)
            .into_iter()
            .filter(|structure| {
                structure_has_layer_at(self.content_registry, structure, layer, cell)
            })
            .map(structure_info)
            .collect())
    }

    /// Returns one placed structure by runtime id.
    ///
    pub fn get_structure(&self, id: PlacedStructureId) -> Option<StructureInfo> {
        self.structures.structure(id).map(structure_info)
    }

    /// Returns all placed structures with the requested kind.
    ///
    pub fn get_structures_by_type(&self, kind: StructureKind) -> Vec<StructureInfo> {
        self.structures
            .iter()
            .filter(|structure| structure.kind == kind)
            .map(structure_info)
            .collect()
    }

    /// Returns the active overlay mode when the caller supplied overlay context.
    ///
    pub fn get_overlay_mode(&self) -> Option<OverlayMode> {
        self.overlay_mode
    }

    /// Returns the currently hovered world cell when the caller supplied input context.
    ///
    pub fn get_hovered_cell(&self) -> Option<UVec2> {
        self.hovered_cell
    }

    fn gas_mixture_from_counts(&self, cell: UVec2, counts: Vec<u32>) -> GasMixture {
        let species = counts
            .into_iter()
            .enumerate()
            .filter(|(_, amount)| *amount > 0)
            .filter_map(|(gas_index, amount)| {
                self.gas_registry
                    .stable_id_by_index(gas_index)
                    .cloned()
                    .map(|substance_id| GasAmount {
                        substance_id,
                        amount,
                    })
            })
            .collect::<Vec<_>>();
        let total_amount = species.iter().map(|entry| entry.amount).sum();
        GasMixture {
            species,
            total_amount,
            velocity: self.gas.velocity(cell.x, cell.y),
        }
    }

    fn ensure_in_bounds(&self, cell: UVec2) -> Result<(), WorldApiError> {
        ensure_in_bounds(cell)
    }
}

/// Mutable world API exposed to plugin systems and event handlers.
///
/// # Fields
/// - `content_registry`: Read-only registry used to validate cell and structure ids before mutations.
/// - `gas_registry`: Read-only gas registry used to resolve substance ids during gas mutations.
/// - `world`: Mutable access to world-cell materials.
/// - `gas`: Mutable access to the free-gas simulation field.
/// - `structures`: Mutable access to placed structures and their layer occupancy maps.
/// - `gpu_state`: Optional GPU upload state that is marked dirty after successful world or gas edits.
pub struct WorldApiMut<'a> {
    pub content_registry: &'a ContentRegistry,
    pub gas_registry: &'a GasRegistry,
    pub world: &'a mut WorldGrid,
    pub gas: &'a mut GasField,
    pub structures: &'a mut PlacedStructureMap,
    pub gpu_state: Option<&'a mut GpuRuntimeState>,
}

impl<'a> WorldApiMut<'a> {
    /// Sets a solid material in one editable world cell.
    ///
    pub fn set_cell_material(
        &mut self,
        cell: UVec2,
        material: CellMaterial,
    ) -> Result<bool, WorldApiError> {
        ensure_in_bounds(cell)?;
        if !is_editable_cell(cell.x, cell.y) {
            return Err(WorldApiError::NotEditable(cell));
        }
        if self.content_registry.cell_by_material(material).is_none() {
            return Err(WorldApiError::UnknownCellMaterial(
                material.as_str().to_string(),
            ));
        }
        let changed = self
            .world
            .set_cell_kind(cell.x, cell.y, CellKind::Solid(material));
        if changed {
            self.gas.clear_cell(cell.x, cell.y);
            self.mark_gpu_dirty();
        }
        Ok(changed)
    }

    /// Removes a solid material from one editable world cell.
    ///
    pub fn remove_cell_material(&mut self, cell: UVec2) -> Result<bool, WorldApiError> {
        ensure_in_bounds(cell)?;
        if !is_editable_cell(cell.x, cell.y) {
            return Err(WorldApiError::NotEditable(cell));
        }
        Ok(self.world.set_empty(cell.x, cell.y))
    }

    /// Places a registered structure using registry-driven layer collision checks.
    ///
    pub fn place_structure(
        &mut self,
        request: StructurePlacement,
    ) -> Result<Option<PlacedStructureId>, WorldApiError> {
        ensure_in_bounds(request.origin)?;
        let descriptor = self
            .content_registry
            .structure_by_kind(request.kind)
            .ok_or_else(|| WorldApiError::UnknownStructureKind(request.kind.as_str().to_string()))?
            .layer_descriptor(request.rotation)
            .clone();
        Ok(self.structures.place_structure_with_descriptor(
            request.kind,
            request.origin,
            request.rotation,
            request.params,
            &descriptor,
            self.world,
        ))
    }

    /// Removes one placed structure by id.
    ///
    pub fn remove_structure(&mut self, id: PlacedStructureId) -> Result<bool, WorldApiError> {
        Ok(self.structures.remove_structure(id))
    }

    /// Removes all structures occupying one cell.
    ///
    pub fn remove_structures_in_cell(
        &mut self,
        cell: UVec2,
    ) -> Result<Vec<PlacedStructureId>, WorldApiError> {
        ensure_in_bounds(cell)?;
        Ok(self.structures.clear_cell(cell.x, cell.y))
    }

    /// Adds gas with a requested cell velocity.
    ///
    pub fn add_gas(
        &mut self,
        cell: UVec2,
        substance: &str,
        amount: u32,
        velocity: Vec2,
    ) -> Result<u32, WorldApiError> {
        ensure_in_bounds(cell)?;
        let gas_index = self.gas_index(substance)?;
        let added = self
            .gas
            .add_particles_with_velocity(cell.x, cell.y, gas_index, amount, velocity, self.world);
        if added > 0 {
            self.mark_gpu_dirty();
        }
        Ok(added)
    }

    /// Sets one gas amount and cell velocity.
    ///
    pub fn set_gas(
        &mut self,
        cell: UVec2,
        substance: &str,
        amount: u32,
        velocity: Vec2,
    ) -> Result<(), WorldApiError> {
        ensure_in_bounds(cell)?;
        let gas_index = self.gas_index(substance)?;
        if self
            .gas
            .set_amount_with_velocity(cell.x, cell.y, gas_index, amount, velocity, self.world)
        {
            self.mark_gpu_dirty();
        }
        Ok(())
    }

    /// Removes gas proportionally from all substances in one cell.
    ///
    pub fn remove_gas(&mut self, cell: UVec2, amount: u32) -> Result<GasMixture, WorldApiError> {
        ensure_in_bounds(cell)?;
        let removed = self
            .gas
            .remove_particles_proportional_counts(cell.x, cell.y, amount, self.world);
        self.gas.recompute_total_density_buffer(self.world);
        self.mark_gpu_dirty();
        Ok(gas_mixture_from_removed(self.gas_registry, removed))
    }

    /// Removes all gas from one cell.
    ///
    pub fn remove_all_gas(&mut self, cell: UVec2) -> Result<(), WorldApiError> {
        ensure_in_bounds(cell)?;
        self.gas.clear_cell(cell.x, cell.y);
        self.mark_gpu_dirty();
        Ok(())
    }

    fn gas_index(&self, substance: &str) -> Result<usize, WorldApiError> {
        self.gas_registry
            .index_of(substance)
            .ok_or_else(|| WorldApiError::UnknownSubstance(substance.to_string()))
    }

    fn mark_gpu_dirty(&mut self) {
        if let Some(gpu_state) = self.gpu_state.as_deref_mut() {
            gpu_state.mark_needs_full_upload();
        }
    }
}

fn ensure_in_bounds(cell: UVec2) -> Result<(), WorldApiError> {
    if cell.x >= WORLD_WIDTH || cell.y >= WORLD_HEIGHT {
        return Err(WorldApiError::OutOfBounds(cell));
    }
    Ok(())
}

fn structure_info(structure: &PlacedStructure) -> StructureInfo {
    StructureInfo {
        id: structure.id,
        kind: structure.kind,
        origin: structure.origin,
        rotation: structure.rotation,
        params: structure.params,
        occupied_cells: structure.occupied_cells(),
    }
}

fn structure_has_layer_at(
    content_registry: &ContentRegistry,
    structure: &PlacedStructure,
    layer: LayerKind,
    cell: UVec2,
) -> bool {
    let Some(descriptor) = descriptor_for_structure(content_registry, structure) else {
        return false;
    };
    descriptor.layers.iter().any(|structure_layer| {
        structure_layer.kind == layer
            && structure_layer.cells.iter().any(|layer_cell| {
                let world_x = structure.origin.x as i32 + layer_cell.local_cell.x;
                let world_y = structure.origin.y as i32 + layer_cell.local_cell.y;
                world_x >= 0 && world_y >= 0 && UVec2::new(world_x as u32, world_y as u32) == cell
            })
    })
}

fn descriptor_for_structure(
    content_registry: &ContentRegistry,
    structure: &PlacedStructure,
) -> Option<StructureDescriptor> {
    content_registry
        .structure_by_kind(structure.kind)
        .map(|descriptor| descriptor.layer_descriptor(structure.rotation).clone())
}

fn gas_mixture_from_removed(gas_registry: &GasRegistry, removed: Vec<u32>) -> GasMixture {
    let species = removed
        .into_iter()
        .enumerate()
        .filter(|(_, amount)| *amount > 0)
        .filter_map(|(gas_index, amount)| {
            gas_registry
                .stable_id_by_index(gas_index)
                .cloned()
                .map(|substance_id| GasAmount {
                    substance_id,
                    amount,
                })
        })
        .collect::<Vec<_>>();
    GasMixture {
        total_amount: species.iter().map(|entry| entry.amount).sum(),
        species,
        velocity: Vec2::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::{StructurePlacement, WorldApi, WorldApiMut};
    use crate::{
        config::GasRegistry,
        plugins::default_plugin,
        simulation::gas::GasField,
        world::{
            grid::WorldGrid,
            structures::{PlacedStructureMap, StructureParams, StructureRotation},
        },
    };
    use bevy::prelude::*;

    fn registry() -> GasRegistry {
        GasRegistry::from_substances(default_plugin::default_substance_definitions())
            .expect("default gas registry")
    }

    #[test]
    fn get_methods_return_world_cell_data() {
        let content = default_plugin::default_content_registry();
        let gas_registry = registry();
        let mut world = WorldGrid::default();
        let structures = PlacedStructureMap::default();
        let mut gas = GasField::from_registry(&gas_registry);
        world.set_solid_with_material(3, 3, default_plugin::brick_cell_material());
        gas.add_particles_with_velocity(4, 4, 0, 25, Vec2::new(1.0, 0.0), &world);

        let api = WorldApi {
            content_registry: &content,
            gas_registry: &gas_registry,
            world: &world,
            gas: &gas,
            structures: &structures,
            overlay_mode: None,
            hovered_cell: Some(UVec2::new(4, 4)),
        };

        assert_eq!(
            api.get_cell_material(UVec2::new(3, 3)).expect("cell"),
            Some(default_plugin::brick_cell_material())
        );
        assert_eq!(
            api.get_cell_gas_amount(UVec2::new(4, 4), "h2")
                .expect("gas amount"),
            25
        );
        assert_eq!(
            api.get_cell_gas_velocity(UVec2::new(4, 4), "h2")
                .expect("gas velocity"),
            Vec2::new(1.0, 0.0)
        );
        assert_eq!(api.get_hovered_cell(), Some(UVec2::new(4, 4)));
    }

    #[test]
    fn mutation_methods_place_structure_and_set_velocity() {
        let content = default_plugin::default_content_registry();
        let gas_registry = registry();
        let mut world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let mut gas = GasField::from_registry(&gas_registry);

        let mut api = WorldApiMut {
            content_registry: &content,
            gas_registry: &gas_registry,
            world: &mut world,
            gas: &mut gas,
            structures: &mut structures,
            gpu_state: None,
        };

        let placed = api
            .place_structure(StructurePlacement {
                kind: default_plugin::vent_structure_kind(),
                origin: UVec2::new(10, 10),
                rotation: StructureRotation::Deg0,
                params: StructureParams::None,
            })
            .expect("place structure");
        assert!(placed.is_some());

        assert_eq!(
            api.add_gas(UVec2::new(11, 11), "o2", 40, Vec2::new(0.0, 1.5))
                .expect("add gas"),
            40
        );
        api.set_gas(UVec2::new(11, 11), "o2", 12, Vec2::new(2.0, 0.0))
            .expect("set gas");
        assert_eq!(api.gas.amount_rounded(11, 11, 1), 12);
        assert_eq!(api.gas.velocity(11, 11), Vec2::new(2.0, 0.0));
    }
}

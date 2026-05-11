use bevy::prelude::Resource;

use crate::plugins::{ContentId, PluginId};

/// One plugin-owned save chunk payload.
///
/// # Fields
/// Public fields of `SaveChunk` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveChunk {
    pub plugin_id: PluginId,
    pub chunk_id: ContentId,
    pub version: u32,
    pub bytes: Vec<u8>,
}

/// In-memory store used while plugins write and read save chunks.
///
/// # Fields
/// Public fields of `SaveChunkStore` are part of the generated SDK reference.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveChunkStore {
    chunks: Vec<SaveChunk>,
}

impl SaveChunkStore {
    /// Writes or replaces one plugin-owned save chunk.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `write_plugin_chunk` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn write_plugin_chunk(
        &mut self,
        plugin_id: PluginId,
        chunk_id: ContentId,
        version: u32,
        bytes: Vec<u8>,
    ) {
        if let Some(existing) = self
            .chunks
            .iter_mut()
            .find(|chunk| chunk.plugin_id == plugin_id && chunk.chunk_id == chunk_id)
        {
            existing.version = version;
            existing.bytes = bytes;
            return;
        }
        self.chunks.push(SaveChunk {
            plugin_id,
            chunk_id,
            version,
            bytes,
        });
    }

    /// Reads one plugin-owned save chunk.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `read_plugin_chunk` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn read_plugin_chunk(
        &self,
        plugin_id: &PluginId,
        chunk_id: &ContentId,
    ) -> Option<&SaveChunk> {
        self.chunks
            .iter()
            .find(|chunk| &chunk.plugin_id == plugin_id && &chunk.chunk_id == chunk_id)
    }

    /// Returns all currently stored plugin chunks.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `chunks` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn chunks(&self) -> &[SaveChunk] {
        &self.chunks
    }
}

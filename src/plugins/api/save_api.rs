use bevy::prelude::Resource;

use crate::plugins::{ContentId, PluginId};

/// One plugin-owned save chunk payload.
///
/// # Fields
/// - `plugin_id`: Plugin that owns the save chunk payload.
/// - `chunk_id`: Stable content id of the save chunk schema.
/// - `version`: Schema version stored alongside the chunk bytes.
/// - `bytes`: Raw serialized payload bytes written by the plugin.
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
/// - `chunks`: Collected plugin-owned save chunks stored for the current save or load pass.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveChunkStore {
    chunks: Vec<SaveChunk>,
}

impl SaveChunkStore {
    /// Writes or replaces one plugin-owned save chunk.
    ///
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
    pub fn chunks(&self) -> &[SaveChunk] {
        &self.chunks
    }
}

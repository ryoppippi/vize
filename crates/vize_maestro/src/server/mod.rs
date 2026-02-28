//! LSP server implementation.
//!
//! This module contains the core LSP server using tower-lsp.

mod capabilities;
mod format;
mod handlers;
mod helpers;
mod state;

pub use capabilities::*;
pub use state::*;

use tower_lsp::Client;

use crate::document::DocumentStore;

/// The Maestro LSP server.
pub struct MaestroServer {
    /// LSP client for sending notifications
    client: Client,
    /// Server state
    state: ServerState,
}

impl MaestroServer {
    /// Create a new Maestro server instance.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: ServerState::new(),
        }
    }

    /// Get the document store.
    pub fn documents(&self) -> &DocumentStore {
        &self.state.documents
    }
}

// Copyright (c) Argument Computer Corporation
// SPDX-License-Identifier: Apache-2.0

//! # Client module
//!
//! This module contains the client for the light client. It is the entrypoint for any needed remote call.
//!
//! ## Sub-modules
//!
//! - `chainweb`: The Chainweb Client is responsible for fetching the data from
//!     the Kadena chain.
//! - `proof_server`: The Proof Server Client is responsible for generating and verifying proofs.

use crate::client::chainweb::ChainwebClient;
use crate::client::error::ClientError;
use crate::client::proof_server::ProofServerClient;
use crate::proofs::{ProofType, ProvingMode};
use kadena_lc_core::types::header::layer::ChainwebLayerHeader;

pub(crate) mod chainweb;
pub mod error;
pub(crate) mod proof_server;
mod utils;

/// The client for the light client. It is the entrypoint for any needed remote call.
#[derive(Debug, Clone)]
pub struct Client {
    chainweb_client: ChainwebClient,
    proof_server_client: ProofServerClient,
}

impl Client {
    /// Create a new client with the given addresses.
    ///
    /// # Arguments
    ///
    /// * `chainweb_node_address: ` - The address of the Chainweb Node API.
    /// * `proof_server_address: ` - The address of the Proof Server.
    ///
    /// # Returns
    ///
    /// A new `Client`.
    pub fn new(chainweb_node_address: &str, proof_server_address: &str) -> Self {
        Self {
            chainweb_client: ChainwebClient::new(chainweb_node_address),
            proof_server_client: ProofServerClient::new(proof_server_address),
        }
    }

    /// Test the connection to all the endpoints.
    ///
    /// # Returns
    ///
    /// A result indicating whether the connections were successful.
    pub async fn test_endpoints(&self) -> Result<(), ClientError> {
        tokio::try_join!(
            self.chainweb_client.test_endpoint(),
            self.proof_server_client.test_endpoint()
        )?;

        Ok(())
    }

    /// Get the layer block headers according to the given block height
    /// and window.
    ///
    /// # Arguments
    ///
    /// * `target_block` - The target block height.
    /// * `block_window` - The window of blocks to fetch.
    ///
    /// # Returns
    ///
    /// The layer block headers.
    pub async fn get_layer_block_headers(
        &self,
        target_block: usize,
        block_window: usize,
    ) -> Result<Vec<ChainwebLayerHeader>, ClientError> {
        self.chainweb_client
            .get_layer_block_headers(target_block, block_window)
            .await
    }

    /// Forwards a request to the proof server to prove the longest chain.
    ///
    /// # Arguments
    ///
    /// * `proving_mode` - The proving mode to use, either STARK or SNARK.
    /// * `layer_block_headers` - The list of Chainweb layer block headers to prove.
    ///
    /// # Returns
    ///
    /// A proof of the longest chain.
    pub async fn prove_longest_chain(
        &self,
        proving_mode: ProvingMode,
        layer_block_headers: Vec<ChainwebLayerHeader>,
    ) -> Result<ProofType, ClientError> {
        self.proof_server_client
            .prove_longest_chain(proving_mode, layer_block_headers)
            .await
    }

    /// Forwards a request to the proof server to verify the longest chain.
    ///
    /// # Arguments
    ///
    /// * `proof` - The proof to verify.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the proof is valid.
    pub async fn verify_longest_chain(&self, proof: ProofType) -> Result<bool, ClientError> {
        self.proof_server_client.verify_longest_chain(proof).await
    }
}

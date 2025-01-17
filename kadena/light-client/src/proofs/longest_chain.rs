// Copyright (c) Argument Computer Corporation
// SPDX-License-Identifier: Apache-2.0

//! # Longest Chain Prover module
//!
//! This module provides the prover implementation for the longest chain change proof. The prover
//! is responsible for generating, executing, proving, and verifying proofs for the light client.

use crate::proofs::error::ProverError;
use crate::proofs::{ProofType, Prover, ProvingMode};
use anyhow::Result;
use getset::CopyGetters;
use kadena_lc_core::crypto::hash::HashValue;
use kadena_lc_core::crypto::U256;
use kadena_lc_core::types::error::TypesError;
use kadena_lc_core::types::header::layer::ChainwebLayerHeader;
use kadena_programs::LONGEST_CHAIN_PROGRAM;
use sphinx_sdk::{
    ProverClient, SphinxProvingKey, SphinxPublicValues, SphinxStdin, SphinxVerifyingKey,
};

/// The prover for the longest chain proof.
pub struct LongestChainProver {
    client: ProverClient,
    keys: (SphinxProvingKey, SphinxVerifyingKey),
}

impl Default for LongestChainProver {
    fn default() -> Self {
        Self::new()
    }
}

impl LongestChainProver {
    /// Create a new `LongestChainProver`.
    ///
    /// # Returns
    ///
    /// A new `LongestChainProver`.
    pub fn new() -> Self {
        let client = ProverClient::new();
        let keys = client.setup(LONGEST_CHAIN_PROGRAM);

        Self { client, keys }
    }

    /// Gets a `SphinxVerifyingKey`.
    ///
    /// # Returns
    ///
    /// A `SphinxVerifyingKey` that can be used for verifying the committee-change proof.
    pub const fn get_vk(&self) -> &SphinxVerifyingKey {
        &self.keys.1
    }
}

/// The input for the sync committee change proof.
#[derive(Debug, Eq, PartialEq)]
pub struct LongestChainIn {
    layer_block_headers: Vec<ChainwebLayerHeader>,
}

impl LongestChainIn {
    /// Create a new `LongestChainIn`.
    ///
    /// # Arguments
    ///
    /// * `layer_block_headers` - The layer block headers.
    ///
    /// # Returns
    ///
    /// A new `CommitteeChangeIn`.
    pub const fn new(layer_block_headers: Vec<ChainwebLayerHeader>) -> Self {
        Self {
            layer_block_headers,
        }
    }

    /// Serialize the `LongestChainIn` struct to bytes.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the serialized `LongestChainIn` struct.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];

        bytes.extend_from_slice(&ChainwebLayerHeader::serialize_list(
            &self.layer_block_headers,
        ));

        bytes
    }

    /// Deserialize a `LongestChainIn` struct from bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The serialized bytes.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the deserialized `LongestChainIn` struct or a `TypesError`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TypesError> {
        Ok(Self {
            layer_block_headers: ChainwebLayerHeader::deserialize_list(bytes)?,
        })
    }
}

/// The output for the sync committee change proof.
#[derive(Debug, Clone, Copy, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct CommitteeChangeOut {
    first_layer_block_header_hash: HashValue,
    target_layer_block_header_hash: HashValue,
    confirmation_work: U256,
}

impl From<&mut SphinxPublicValues> for CommitteeChangeOut {
    fn from(public_values: &mut SphinxPublicValues) -> Self {
        let confirmation_work = U256::from_little_endian(&public_values.read::<[u8; 32]>());
        let first_layer_block_header_hash = HashValue::new(public_values.read::<[u8; 32]>());
        let target_layer_block_header_hash = HashValue::new(public_values.read::<[u8; 32]>());

        Self {
            confirmation_work,
            first_layer_block_header_hash,
            target_layer_block_header_hash,
        }
    }
}

impl Prover for LongestChainProver {
    const PROGRAM: &'static [u8] = LONGEST_CHAIN_PROGRAM;
    type Error = ProverError;
    type StdIn = LongestChainIn;
    type StdOut = CommitteeChangeOut;

    fn generate_sphinx_stdin(&self, inputs: &Self::StdIn) -> Result<SphinxStdin, Self::Error> {
        let mut stdin = SphinxStdin::new();
        stdin.write(&ChainwebLayerHeader::serialize_list(
            &inputs.layer_block_headers,
        ));
        Ok(stdin)
    }

    fn execute(&self, inputs: &Self::StdIn) -> Result<Self::StdOut, Self::Error> {
        sphinx_sdk::utils::setup_logger();

        let stdin = self.generate_sphinx_stdin(inputs)?;

        let (mut public_values, _) = self
            .client
            .execute(Self::PROGRAM, stdin)
            .run()
            .map_err(|err| ProverError::Execution { source: err.into() })?;

        Ok(CommitteeChangeOut::from(&mut public_values))
    }

    fn prove(&self, inputs: &Self::StdIn, mode: ProvingMode) -> Result<ProofType, Self::Error> {
        let stdin = self.generate_sphinx_stdin(inputs)?;

        match mode {
            ProvingMode::STARK => self
                .client
                .prove(&self.keys.0, stdin)
                .run()
                .map_err(|err| ProverError::Proving {
                    proof_type: mode.into(),
                    source: err.into(),
                })
                .map(ProofType::STARK),
            ProvingMode::SNARK => self
                .client
                .prove(&self.keys.0, stdin)
                .plonk()
                .run()
                .map_err(|err| ProverError::Proving {
                    proof_type: mode.into(),
                    source: err.into(),
                })
                .map(ProofType::SNARK),
        }
    }

    fn verify(&self, proof: &ProofType) -> Result<(), Self::Error> {
        let vk = &self.keys.1;

        match proof {
            ProofType::STARK(proof) | ProofType::SNARK(proof) => self
                .client
                .verify(proof, vk)
                .map_err(|err| ProverError::Verification { source: err.into() }),
        }
    }
}

#[cfg(all(test, feature = "kadena"))]
mod test {
    use super::*;
    use kadena_lc_core::test_utils::get_layer_block_headers;

    #[test]
    fn test_execute_committee_change() {
        let headers = get_layer_block_headers();

        let prover = LongestChainProver::new();

        let new_period_inputs = LongestChainIn {
            layer_block_headers: headers.clone(),
        };

        let new_period_output = prover.execute(&new_period_inputs).unwrap();

        let confirmation_work = ChainwebLayerHeader::cumulative_produced_work(
            headers[headers.len() / 2..headers.len() - 1].to_vec(),
        )
        .expect("Should be able to calculate cumulative work");

        assert_eq!(new_period_output.confirmation_work, confirmation_work,);
        assert_eq!(
            new_period_output.first_layer_block_header_hash,
            headers
                .first()
                .expect("Should have a first header")
                .header_root()
                .expect("Should have a header root"),
        );
        assert_eq!(
            new_period_output.target_layer_block_header_hash,
            headers[headers.len() / 2]
                .header_root()
                .expect("Should have a header root"),
        );
    }

    #[test]
    #[ignore = "This test is too slow for CI"]
    fn test_prove_stark_committee_change() {
        use std::time::Instant;

        let layer_block_headers = get_layer_block_headers();

        let prover = LongestChainProver::new();

        let new_period_inputs = LongestChainIn {
            layer_block_headers,
        };

        println!("Starting STARK proving for sync committee change...");
        let start = Instant::now();

        let _ = prover
            .prove(&new_period_inputs, ProvingMode::STARK)
            .unwrap();
        println!("Proving took {:?}", start.elapsed());
    }

    #[test]
    #[ignore = "This test is too slow for CI"]
    fn test_prove_snark_committee_change() {
        use std::time::Instant;

        let layer_block_headers = get_layer_block_headers();

        let prover = LongestChainProver::new();

        let new_period_inputs = LongestChainIn {
            layer_block_headers,
        };

        println!("Starting SNARK proving for sync committee change...");
        let start = Instant::now();

        let _ = prover
            .prove(&new_period_inputs, ProvingMode::SNARK)
            .unwrap();
        println!("Proving took {:?}", start.elapsed());
    }
}

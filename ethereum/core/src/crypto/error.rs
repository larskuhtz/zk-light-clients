// Copyright (c) Argument Computer Corporation
// SPDX-License-Identifier: APACHE-2.0

use thiserror::Error;

/// The error type for the `crypto` module.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Internal error occurred: {source}")]
    Internal {
        #[source]
        source: Box<dyn std::error::Error + Sync + Send>,
    },
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Invalid digest length: expected {expected}, got {actual}")]
    DigestLength { expected: usize, actual: usize },
}

// Copyright 2020 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0

//!
//! Types that represent errors in mina serialization and deserialization

/// Type that represents errors in mina serialization and deserialization
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error decoding base58 string
    #[error("Error decoding base58 string: {0}")]
    Base58DecodeError(#[from] bs58::decode::Error),

    /// Custom error
    #[error("Custom error: {0}")]
    Custom(String),
}

/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

use crate::RuntimeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    RuntimeError(Box<RuntimeError>),
    #[error("Inventory not found.")]
    InventoryNotFound(String, String),
}

impl From<RuntimeError> for ApiError {
    fn from(error: RuntimeError) -> Self {
        ApiError::RuntimeError(error.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ApiError, RuntimeError};
    use std::env::VarError;

    #[test]
    fn runtime_api_error() {
        match ApiError::from(RuntimeError::ClientConfigLoadingError(VarError::NotPresent)) {
            ApiError::RuntimeError(_) => {}
            _ => {
                panic!("Invalid error type.");
            }
        }
    }
}

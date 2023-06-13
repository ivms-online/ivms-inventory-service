/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

mod api_error;
mod inventory_dao;
mod lambda;
mod model;
mod runtime_error;

pub use crate::api_error::ApiError;
pub use crate::inventory_dao::InventoryDao;
pub use crate::lambda::run_lambda;
pub use crate::model::{DynamoResultsPage, Inventory};
pub use crate::runtime_error::RuntimeError;

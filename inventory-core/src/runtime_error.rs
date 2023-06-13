/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

use aws_sdk_dynamodb::operation::delete_item::DeleteItemError;
use aws_sdk_dynamodb::operation::get_item::GetItemError;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::operation::query::QueryError;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_smithy_http::result::SdkError;
use serde_dynamo::Error as SerializationError;
use std::env::VarError;
use std::fmt::{Debug, Display, Formatter, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    ClientConfigLoadingError(VarError),
    DeleteItemError(#[from] SdkError<DeleteItemError>),
    GetItemError(#[from] SdkError<GetItemError>),
    PutItemError(#[from] SdkError<PutItemError>),
    QueryError(#[from] SdkError<QueryError>),
    DataError(AttributeValue, String),
    SerializationError(#[from] SerializationError),
}

impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{self:?}")
    }
}

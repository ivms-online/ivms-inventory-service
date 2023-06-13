/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc = "License entity."]
pub struct Inventory {
    #[doc = "Owner ID."]
    pub customer_id: Uuid,
    #[doc = "Vessel ID."]
    pub vessel_id: Uuid,
    #[doc = "Inventory type."]
    pub inventory_type: String,
    #[doc = "Inventory ID (within given type)."]
    pub inventory_id: String,
    #[doc = "Serial number."]
    pub serial_number: Option<String>,
    #[doc = "AWS Systems Manager identifier."]
    pub aws_instance_id: Option<String>,
    #[doc = "Date when inventory was added."]
    pub created_at: DateTime<FixedOffset>,
}

pub struct DynamoResultsPage<T, K> {
    pub items: Vec<T>,
    pub last_evaluated_key: Option<K>,
}

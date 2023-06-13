/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

use chrono::Utc;
use inventory_core::{run_lambda, ApiError, Inventory, InventoryDao};
use lambda_runtime::{Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use tokio::main as tokio_main;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    customer_id: Uuid,
    vessel_id: Uuid,
    inventory_type: String,
    inventory_id: String,
    serial_number: Option<String>,
    aws_instance_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    inventory_type: String,
    inventory_id: String,
}

#[tokio_main]
async fn main() -> Result<(), Error> {
    let dao = &InventoryDao::load_from_env().await?;

    run_lambda!(move |event: LambdaEvent<Request>| async move {
        dao.create_inventory(Inventory {
            customer_id: event.payload.customer_id,
            vessel_id: event.payload.vessel_id,
            inventory_type: event.payload.inventory_type.clone(),
            inventory_id: event.payload.inventory_id.clone(),
            serial_number: event.payload.serial_number,
            aws_instance_id: event.payload.aws_instance_id,
            created_at: Utc::now().fixed_offset(),
        })
        .await?;

        Ok::<Response, ApiError>(Response {
            inventory_type: event.payload.inventory_type,
            inventory_id: event.payload.inventory_id,
        })
    })
}

#[cfg(test)]
mod tests {
    use crate::Request;
    use chrono::{DateTime, FixedOffset};
    use serde_json::from_str;
    use uuid::{uuid, Uuid};

    const CUSTOMER_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    const VESSEL_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    const INVENTORY_TYPE: &str = "station";
    const INVENTORY_ID: &str = "123";
    const SERIAL_NUMBER: &str = "abc";

    #[test]
    fn deserialize_request() {
        let input = format!(
            "{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\",\"inventoryType\":\"{INVENTORY_TYPE}\",\"inventoryId\":\"{INVENTORY_ID}\"}}"
        );
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(INVENTORY_TYPE, request.inventory_type);
        assert!(request.serial_number.is_none());
    }

    #[test]
    fn deserialize_request_optional() {
        let input = format!("{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\",\"inventoryType\":\"{INVENTORY_TYPE}\",\"inventoryId\":\"{INVENTORY_ID}\",\"serialNumber\":\"{SERIAL_NUMBER}\"}}");
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(Some(SERIAL_NUMBER.to_string()), request.serial_number);
    }
}

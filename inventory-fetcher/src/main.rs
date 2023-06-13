/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

use chrono::{DateTime, FixedOffset};
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    inventory_type: String,
    inventory_id: String,
    serial_number: Option<String>,
    aws_instance_id: Option<String>,
    created_at: DateTime<FixedOffset>,
}

impl From<Inventory> for Response {
    fn from(model: Inventory) -> Self {
        Self {
            inventory_type: model.inventory_type,
            inventory_id: model.inventory_id,
            serial_number: model.serial_number,
            aws_instance_id: model.aws_instance_id,
            created_at: model.created_at,
        }
    }
}

#[tokio_main]
async fn main() -> Result<(), Error> {
    let dao = &InventoryDao::load_from_env().await?;

    run_lambda!(move |event: LambdaEvent<Request>| async move {
        match dao
            .get_inventory(
                event.payload.customer_id,
                event.payload.vessel_id,
                event.payload.inventory_type.clone(),
                event.payload.inventory_id.clone(),
            )
            .await?
        {
            None => Err(ApiError::InventoryNotFound(
                event.payload.inventory_type,
                event.payload.inventory_id,
            )),
            Some(license) => Ok(Response::from(license)),
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::{Request, Response};
    use chrono::{FixedOffset, TimeZone, Utc};
    use inventory_core::Inventory;
    use serde_json::{from_str, to_string};
    use uuid::{uuid, Uuid};

    const CUSTOMER_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    const VESSEL_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    const INVENTORY_TYPE: &str = "pc";
    const INVENTORY_ID: &str = "0";
    const SERIAL_NUMBER: &str = "abc";

    #[test]
    fn deserialize_request() {
        let input = format!(
            "{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\",\"inventoryType\":\"{INVENTORY_TYPE}\",\"inventoryId\":\"{INVENTORY_ID}\"}}"
        );
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(VESSEL_ID, request.vessel_id);
        assert_eq!(INVENTORY_TYPE, request.inventory_type);
        assert_eq!(INVENTORY_ID, request.inventory_id);
    }

    #[test]
    fn serialize_response() {
        let created_at = Utc
            .with_ymd_and_hms(2011, 1, 30, 13, 58, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(3600).unwrap());

        let output = to_string(&Response {
            inventory_type: INVENTORY_TYPE.to_string(),
            inventory_id: INVENTORY_ID.to_string(),
            serial_number: Some(SERIAL_NUMBER.to_string()),
            aws_instance_id: None,
            created_at,
        })
        .unwrap();

        assert!(output.contains(&format!("\"{INVENTORY_ID}\"")));
        assert!(output.contains(&format!("\"{SERIAL_NUMBER}\"")));
        assert!(output.contains(&format!("\"2011-01-30T14:58:00+01:00\"")));
    }

    #[test]
    fn response_from_model() {
        let created_at = Utc
            .with_ymd_and_hms(2015, 7, 2, 1, 20, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(7200).unwrap());

        let response = Response::from(Inventory {
            customer_id: CUSTOMER_ID,
            vessel_id: VESSEL_ID,
            inventory_type: INVENTORY_TYPE.to_string(),
            inventory_id: INVENTORY_ID.to_string(),
            serial_number: Some(SERIAL_NUMBER.to_string()),
            aws_instance_id: None,
            created_at,
        });

        assert_eq!(INVENTORY_TYPE, response.inventory_type);
        assert_eq!(Some(SERIAL_NUMBER.to_string()), response.serial_number);
        assert!(response.aws_instance_id.is_none());
    }
}

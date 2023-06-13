/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(future_join)]

use chrono::{DateTime, FixedOffset};
use inventory_core::{run_lambda, DynamoResultsPage, Inventory, InventoryDao};
use lambda_runtime::{Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use tokio::main as tokio_main;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    customer_id: Uuid,
    vessel_id: Uuid,
    page_token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InventoryResponse {
    inventory_type: String,
    inventory_id: String,
    serial_number: Option<String>,
    aws_instance_id: Option<String>,
    created_at: DateTime<FixedOffset>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    inventory: Vec<InventoryResponse>,
    page_token: Option<String>,
}

impl From<Inventory> for InventoryResponse {
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

impl From<DynamoResultsPage<Inventory, String>> for Response {
    fn from(value: DynamoResultsPage<Inventory, String>) -> Self {
        Self {
            inventory: value.items.into_iter().map(InventoryResponse::from).collect(),
            page_token: value.last_evaluated_key,
        }
    }
}

#[tokio_main]
async fn main() -> Result<(), Error> {
    let dao = &InventoryDao::load_from_env().await?;

    run_lambda!(move |event: LambdaEvent<Request>| async move {
        dao.list_inventory(
            event.payload.customer_id,
            event.payload.vessel_id,
            event.payload.page_token,
        )
        .await
        .map(Response::from)
    })
}

#[cfg(test)]
mod tests {
    use crate::{InventoryResponse, Request, Response};
    use chrono::{FixedOffset, TimeZone, Utc};
    use inventory_core::{DynamoResultsPage, Inventory};
    use serde_json::{from_str, to_string};
    use uuid::{uuid, Uuid};

    const CUSTOMER_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    const VESSEL_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    const INVENTORY_TYPE: &str = "pc";
    const INVENTORY_ID: &str = "0";
    const SERIAL_NUMBER: &str = "q1w2e3r4";
    const PAGE_TOKEN: &str = "abc";

    #[test]
    fn deserialize_request() {
        let input =
            format!("{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\",\"pageToken\":\"{PAGE_TOKEN}\"}}");
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(VESSEL_ID, request.vessel_id);
        assert_eq!(Some(PAGE_TOKEN.to_string()), request.page_token);
    }

    #[test]
    fn deserialize_request_no_page() {
        let input = format!("{{\"customerId\":\"{CUSTOMER_ID}\",\"vesselId\":\"{VESSEL_ID}\"}}");
        let request: Request = from_str(&input).unwrap();

        assert_eq!(CUSTOMER_ID, request.customer_id);
        assert_eq!(VESSEL_ID, request.vessel_id);
        assert!(request.page_token.is_none());
    }

    #[test]
    fn serialize_response() {
        let created_at = Utc
            .with_ymd_and_hms(2009, 3, 23, 10, 0, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(7200).unwrap());

        let output = to_string(&Response {
            inventory: vec![InventoryResponse {
                inventory_type: INVENTORY_TYPE.to_string(),
                inventory_id: INVENTORY_ID.to_string(),
                serial_number: Some(SERIAL_NUMBER.to_string()),
                aws_instance_id: None,
                created_at,
            }],
            page_token: Some(PAGE_TOKEN.to_string()),
        })
        .unwrap();

        assert!(output.contains(&format!("\"{SERIAL_NUMBER}\"")));
        assert!(output.contains(&format!("\"{PAGE_TOKEN}\"")));
    }

    #[test]
    fn serialize_response_no_page() {
        let created_at = Utc
            .with_ymd_and_hms(2017, 11, 11, 16, 0, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(7200).unwrap());

        let output = to_string(&Response {
            inventory: vec![InventoryResponse {
                inventory_type: INVENTORY_TYPE.to_string(),
                inventory_id: INVENTORY_ID.to_string(),
                serial_number: None,
                aws_instance_id: None,
                created_at,
            }],
            page_token: None,
        })
        .unwrap();

        assert!(!output.contains(&format!("\"page_token\":")));
    }

    #[test]
    fn response_license_from_model() {
        let created_at = Utc
            .with_ymd_and_hms(2015, 7, 2, 1, 20, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(7200).unwrap());

        let response = InventoryResponse::from(Inventory {
            customer_id: CUSTOMER_ID,
            vessel_id: VESSEL_ID,
            inventory_type: INVENTORY_TYPE.to_string(),
            inventory_id: INVENTORY_ID.to_string(),
            serial_number: Some(SERIAL_NUMBER.to_string()),
            aws_instance_id: None,
            created_at,
        });

        assert_eq!(Some(SERIAL_NUMBER.to_string()), response.serial_number);
    }

    #[test]
    fn response_from_model() {
        let created_at = Utc
            .with_ymd_and_hms(2011, 1, 30, 13, 58, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(3600).unwrap());

        let response = Response::from(DynamoResultsPage {
            items: vec![Inventory {
                customer_id: CUSTOMER_ID,
                vessel_id: VESSEL_ID,
                inventory_type: INVENTORY_TYPE.to_string(),
                inventory_id: INVENTORY_ID.to_string(),
                serial_number: Some(SERIAL_NUMBER.to_string()),
                aws_instance_id: None,
                created_at,
            }],
            last_evaluated_key: Some(PAGE_TOKEN.to_string()),
        });

        assert_eq!(1, response.inventory.len());
        assert_eq!(Some(SERIAL_NUMBER.to_string()), response.inventory[0].serial_number);
        assert_eq!(Some(PAGE_TOKEN.to_string()), response.page_token);
    }
}

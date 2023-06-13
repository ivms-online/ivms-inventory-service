/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

#![feature(async_closure, future_join)]

use aws_config::load_from_env;
use aws_sdk_dynamodb::types::AttributeValue::S;
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_lambda::error::SdkError;
use aws_sdk_lambda::operation::invoke::{InvokeError, InvokeOutput};
use aws_sdk_lambda::Client as LambdaClient;
use aws_smithy_types::Blob;
use cucumber::{given, then, when, World};
use futures::future::join_all;
use serde_json::{from_slice, json, to_vec, Value};
use std::collections::HashMap;
use std::env::{var, VarError};
use std::future::join;
use tokio::main as tokio_main;

macro_rules! serialize_blob {
    ($($data:tt)+) => {
        Blob::new(
            to_vec(&json!($($data)+)).unwrap()
        )
    };
}

#[derive(World, Debug)]
#[world(init = Self::new)]
struct TestWorld {
    // initialization scope
    inventory_table: String,
    creator_lambda: String,
    deleter_lambda: String,
    fetcher_lambda: String,
    lister_lambda: String,
    dynamodb: DynamoDbClient,
    lambda: LambdaClient,
    // test run scope
    cleanup_keys: Vec<(String, String, String, String)>,
    invoke_response: Option<Result<InvokeOutput, SdkError<InvokeError>>>,
    customer_id: Option<String>,
    vessel_id: Option<String>,
    inventory_type: Option<String>,
    inventory_id: Option<String>,
}

impl TestWorld {
    async fn new() -> Result<Self, VarError> {
        let config = &load_from_env().await;

        Ok(Self {
            inventory_table: var("INVENTORY_TABLE")?,
            creator_lambda: var("CREATOR_LAMBDA")?,
            deleter_lambda: var("DELETER_LAMBDA")?,
            fetcher_lambda: var("FETCHER_LAMBDA")?,
            lister_lambda: var("LISTER_LAMBDA")?,
            dynamodb: DynamoDbClient::new(config),
            lambda: LambdaClient::new(config),
            cleanup_keys: vec![],
            invoke_response: None,
            customer_id: None,
            vessel_id: None,
            inventory_type: None,
            inventory_id: None,
        })
    }
}

async fn delete_inventory(
    world: &TestWorld,
    customer_id: &Option<String>,
    vessel_id: &Option<String>,
    inventory_type: &Option<String>,
    inventory_id: &Option<String>,
) {
    if let (Some(customer_id), Some(vessel_id), Some(inventory_type), Some(inventory_id)) =
        (customer_id, vessel_id, inventory_type, inventory_id)
    {
        world
            .dynamodb
            .delete_item()
            .table_name(world.inventory_table.as_str())
            .key("customerAndVesselId", S(format!("{customer_id}:{vessel_id}")))
            .key("inventoryKey", S(format!("{inventory_type}:{inventory_id}")))
            .send()
            .await
            .unwrap();
    }
}

async fn list_inventory(
    world: &TestWorld,
    customer_id: String,
    vessel_id: String,
    page_token: Option<String>,
) -> Result<InvokeOutput, SdkError<InvokeError>> {
    world
        .lambda
        .invoke()
        .function_name(world.lister_lambda.to_string())
        .payload(serialize_blob!({
            "customerId": customer_id,
            "vesselId": vessel_id,
            "pageToken": page_token,
        }))
        .send()
        .await
}

fn extract_list(response: &Option<Result<InvokeOutput, SdkError<InvokeError>>>) -> Vec<Value> {
    let response: HashMap<String, Value> = from_slice(
        response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    response["inventory"].as_array().unwrap().to_owned()
}

#[tokio_main]
async fn main() {
    TestWorld::cucumber()
        .after(|_feature, _rule, _scenario, _finished, world| {
            Box::pin(async move {
                if let Some(&mut ref cleanup) = world {
                    let tasks = cleanup.cleanup_keys.iter().map(async move |key| {
                        delete_inventory(
                            &cleanup,
                            &Some(key.0.clone()),
                            &Some(key.1.clone()),
                            &Some(key.2.clone()),
                            &Some(key.3.clone()),
                        )
                        .await
                    });

                    join!(
                        join_all(tasks),
                        delete_inventory(
                            &cleanup,
                            &cleanup.customer_id,
                            &cleanup.vessel_id,
                            &cleanup.inventory_type,
                            &cleanup.inventory_id,
                        ),
                    )
                    .await;
                }
            })
        })
        .run_and_exit("tests/features")
        .await;
}

// Given …

#[given(
    expr = "There is an inventory {string} of type {string} for vessel {string} of customer {string} with serial number {string}, AWS instance ID {string} and creation date {string}"
)]
async fn there_is_an_inventory(
    world: &mut TestWorld,
    inventory_id: String,
    inventory_type: String,
    vessel_id: String,
    customer_id: String,
    serial_number: String,
    aws_instance_id: String,
    created_at: String,
) {
    world.cleanup_keys.push((
        customer_id.clone(),
        vessel_id.clone(),
        inventory_type.clone(),
        inventory_id.clone(),
    ));

    world
        .dynamodb
        .put_item()
        .table_name(world.inventory_table.as_str())
        .item("customerAndVesselId", S(format!("{customer_id}:{vessel_id}")))
        .item("inventoryKey", S(format!("{inventory_type}:{inventory_id}")))
        .item("customerId", S(customer_id))
        .item("vesselId", S(vessel_id))
        .item("inventoryType", S(inventory_type))
        .item("inventoryId", S(inventory_id))
        .item("serialNumber", S(serial_number))
        .item("awsInstanceId", S(aws_instance_id))
        .item("createdAt", S(created_at))
        .send()
        .await
        .unwrap();
}

#[given(expr = "There is no inventory {string} of type {string} for vessel {string} of customer {string}")]
async fn there_is_no_inventory(
    world: &mut TestWorld,
    inventory_id: String,
    inventory_type: String,
    vessel_id: String,
    customer_id: String,
) {
    delete_inventory(
        world,
        &Some(customer_id),
        &Some(vessel_id),
        &Some(inventory_type),
        &Some(inventory_id),
    )
    .await;
}

// When …

#[when(expr = "I delete inventory {string} of type {string} for vessel {string} of customer {string}")]
async fn i_delete_inventory(
    world: &mut TestWorld,
    inventory_id: String,
    inventory_type: String,
    vessel_id: String,
    customer_id: String,
) {
    world.invoke_response = Some(
        world
            .lambda
            .invoke()
            .function_name(world.deleter_lambda.to_string())
            .payload(serialize_blob!({
                "customerId": customer_id,
                "vesselId": vessel_id,
                "inventoryType": inventory_type,
                "inventoryId": inventory_id,
            }))
            .send()
            .await,
    );
}

#[when(
    expr = "I create inventory {string} of type {string} for vessel {string} of customer {string} with serial number {string} and AWS instance ID {string}"
)]
async fn i_create_inventory(
    world: &mut TestWorld,
    inventory_id: String,
    inventory_type: String,
    vessel_id: String,
    customer_id: String,
    serial_number: String,
    aws_instance_id: String,
) {
    world.invoke_response = Some(
        world
            .lambda
            .invoke()
            .function_name(world.creator_lambda.to_string())
            .payload(serialize_blob!({
                "customerId": customer_id,
                "vesselId": vessel_id,
                "inventoryType": inventory_type,
                "inventoryId": inventory_id,
                "serialNumber": serial_number,
                "awsInstanceId": aws_instance_id,
            }))
            .send()
            .await,
    );

    world.customer_id = Some(customer_id);
    world.vessel_id = Some(vessel_id);
}

#[when(expr = "I fetch inventory {string} of type {string} for vessel {string} of customer {string}")]
async fn i_fetch_inventory(
    world: &mut TestWorld,
    inventory_id: String,
    inventory_type: String,
    vessel_id: String,
    customer_id: String,
) {
    world.invoke_response = Some(
        world
            .lambda
            .invoke()
            .function_name(world.fetcher_lambda.to_string())
            .payload(serialize_blob!({
                "customerId": customer_id,
                "vesselId": vessel_id,
                "inventoryType": inventory_type,
                "inventoryId": inventory_id,
            }))
            .send()
            .await,
    );
}

#[when(expr = "I list inventory for vessel {string} of customer {string}")]
async fn i_list_inventory(world: &mut TestWorld, vessel_id: String, customer_id: String) {
    world.invoke_response = Some(list_inventory(world, customer_id, vessel_id, None).await);
}

#[when(expr = "I list inventory for vessel {string} of customer {string} with page token {string}")]
async fn i_list_inventory_page(world: &mut TestWorld, vessel_id: String, customer_id: String, page_token: String) {
    world.invoke_response = Some(list_inventory(world, customer_id, vessel_id, Some(page_token)).await);
}

// Then …

#[then(expr = "Inventory {string} of type {string} for vessel {string} of customer {string} does not exist")]
async fn inventory_does_not_exist(
    world: &mut TestWorld,
    inventory_id: String,
    inventory_type: String,
    vessel_id: String,
    customer_id: String,
) {
    assert!(world
        .dynamodb
        .get_item()
        .table_name(world.inventory_table.as_str())
        .key("customerAndVesselId", S(format!("{customer_id}:{vessel_id}")))
        .key("inventoryKey", S(format!("{inventory_type}:{inventory_id}")))
        .send()
        .await
        .unwrap()
        .item
        .is_none())
}

#[then(expr = "I get {string} API error response")]
async fn i_get_api_error(world: &mut TestWorld, message: String) {
    let response: HashMap<String, String> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(message, response["errorMessage"]);
}

#[then(expr = "I can read inventory type as {string}")]
async fn i_can_read_inventory_type(world: &mut TestWorld, inventory_type: String) {
    let response: HashMap<String, Value> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(inventory_type.as_str(), response["inventoryType"].as_str().unwrap());
}

#[then(expr = "I can read inventory ID as {string}")]
async fn i_can_read_inventory_id(world: &mut TestWorld, inventory_id: String) {
    let response: HashMap<String, Value> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
        .unwrap();

    assert_eq!(inventory_id.as_str(), response["inventoryId"].as_str().unwrap());
}

#[then(expr = "I can read inventory serial number as {string}")]
async fn i_can_read_inventory_serial_number(world: &mut TestWorld, serial_number: String) {
    let response: HashMap<String, Value> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(serial_number.as_str(), response["serialNumber"].as_str().unwrap());
}

#[then(expr = "I can read inventory AWS instance ID as {string}")]
async fn i_can_read_inventory_aws_instance_id(world: &mut TestWorld, aws_instance_id: String) {
    let response: HashMap<String, Value> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(aws_instance_id.as_str(), response["awsInstanceId"].as_str().unwrap());
}

#[then(expr = "I can read inventory creation date as {string}")]
async fn i_can_read_inventory_creation_date(world: &mut TestWorld, created_at: String) {
    let response: HashMap<String, Value> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    assert_eq!(created_at.as_str(), response["createdAt"].as_str().unwrap());
}

#[then("I can read inventory key")]
async fn i_can_read_inventory_key_after_create(world: &mut TestWorld) {
    let response: HashMap<String, String> = from_slice(
        world
            .invoke_response
            .as_ref()
            .and_then(|response| response.as_ref().ok())
            .and_then(|response| response.payload())
            .unwrap()
            .as_ref(),
    )
    .unwrap();

    world.inventory_type = Some(response["inventoryType"].clone());
    assert!(world.inventory_type.is_some());
    world.inventory_id = Some(response["inventoryId"].clone());
    assert!(world.inventory_id.is_some());
}

#[then(expr = "Inventory with that key exists with serial number {string}, AWS instance ID {string} and creation date")]
async fn inventory_with_that_key_exists(world: &mut TestWorld, serial_number: String, aws_instance_id: String) {
    let customer_id = world.customer_id.clone().unwrap();
    let vessel_id = world.vessel_id.clone().unwrap();

    let inventory_type = world.inventory_type.clone().unwrap();
    let inventory_id = world.inventory_id.clone().unwrap();

    let inventory = world
        .dynamodb
        .get_item()
        .table_name(world.inventory_table.as_str())
        .key("customerAndVesselId", S(format!("{customer_id}:{vessel_id}")))
        .key("inventoryKey", S(format!("{inventory_type}:{inventory_id}")))
        .send()
        .await
        .unwrap()
        .item;
    assert!(inventory.is_some());
    assert_eq!(
        serial_number,
        *inventory
            .as_ref()
            .and_then(|item| item["serialNumber"].as_s().ok())
            .unwrap()
    );
    assert_eq!(
        aws_instance_id,
        *inventory
            .as_ref()
            .and_then(|item| item["awsInstanceId"].as_s().ok())
            .unwrap()
    );
    assert!(inventory
        .as_ref()
        .and_then(|item| item["createdAt"].as_s().ok())
        .is_some());
}

#[then(expr = "I can read list of {int} inventories")]
async fn i_can_read_list_of_inventories(world: &mut TestWorld, count: usize) {
    let inventories = extract_list(&world.invoke_response);

    assert_eq!(count, inventories.len());
}

#[then(expr = "Inventory at position {int} has ID {string} and type {string}")]
async fn inventory_at_position_has_key(
    world: &mut TestWorld,
    position: usize,
    inventory_id: String,
    inventory_type: String,
) {
    let inventory = extract_list(&world.invoke_response);
    let entry = inventory[position].as_object().unwrap();

    assert_eq!(inventory_type, entry["inventoryType"].as_str().unwrap());
    assert_eq!(inventory_id, entry["inventoryId"].as_str().unwrap());
}

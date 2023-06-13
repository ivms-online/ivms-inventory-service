/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

use crate::model::{DynamoResultsPage, Inventory};
use crate::runtime_error::RuntimeError;
use std::collections::HashMap;

use aws_config::load_from_env;
use aws_sdk_dynamodb::types::AttributeValue::S;
use aws_sdk_dynamodb::Client;
use serde_dynamo::{from_item, from_items, to_item};
use std::env::var;
use tracing::{Instrument, Span};
use uuid::Uuid;
use xray::aws_metadata;

pub struct InventoryDao {
    client: Box<Client>,
    table_name: String,
}

#[inline(always)]
fn hash_key_of(customer_id: &Uuid, vessel_id: &Uuid) -> String {
    format!("{customer_id}:{vessel_id}")
}

#[inline(always)]
fn sort_key_of(inventory_type: &String, inventory_id: &String) -> String {
    format!("{inventory_type}:{inventory_id}")
}

/**
Required environment variables:
<dl>
    <dt><code>INVENTORY_TABLE</code></dt>
    <dd>Name of DynamoDB licenses table.</dd>
</dl>
 */
impl InventoryDao {
    pub async fn load_from_env() -> Result<Self, RuntimeError> {
        let config = &load_from_env().await;

        var("INVENTORY_TABLE")
            .map(|table_name| {
                let client = Client::new(config);
                Self::new(client, table_name)
            })
            .map_err(RuntimeError::ClientConfigLoadingError)
    }

    pub fn new(client: Client, table_name: String) -> Self {
        Self {
            client: Box::new(client),
            table_name,
        }
    }

    pub async fn create_inventory(&self, inventory: Inventory) -> Result<(), RuntimeError> {
        let hash_key = hash_key_of(&inventory.customer_id, &inventory.vessel_id);
        let sort_key = sort_key_of(&inventory.inventory_type, &inventory.inventory_id);

        self.client
            .put_item()
            .table_name(self.table_name.as_str())
            .set_item(Some(to_item(inventory)?))
            .item("customerAndVesselId", S(hash_key))
            .item("inventoryKey", S(sort_key))
            .send()
            .instrument(self.instrumentation())
            .await?;
        Ok(())
    }

    pub async fn list_inventory(
        &self,
        customer_id: Uuid,
        vessel_id: Uuid,
        page_token: Option<String>,
    ) -> Result<DynamoResultsPage<Inventory, String>, RuntimeError> {
        let hash_key = hash_key_of(&customer_id, &vessel_id);

        let results = self
            .client
            .query()
            .table_name(self.table_name.as_str())
            .key_condition_expression("customerAndVesselId = :customerAndVesselId")
            .expression_attribute_values(":customerAndVesselId", S(hash_key.clone()))
            .set_exclusive_start_key(page_token.map(|inventory_key| {
                HashMap::from([
                    ("customerAndVesselId".into(), S(hash_key)),
                    ("inventoryKey".into(), S(inventory_key)),
                ])
            }))
            .send()
            .instrument(self.instrumentation())
            .await?;

        Ok(DynamoResultsPage {
            last_evaluated_key: results
                .last_evaluated_key()
                .and_then(|key| key["inventoryKey"].as_s().ok())
                .cloned(),
            items: if let Some(items) = results.items {
                from_items(items)?
            } else {
                vec![]
            },
        })
    }

    pub async fn get_inventory(
        &self,
        customer_id: Uuid,
        vessel_id: Uuid,
        inventory_type: String,
        inventory_id: String,
    ) -> Result<Option<Inventory>, RuntimeError> {
        self.client
            .get_item()
            .table_name(self.table_name.as_str())
            .key("customerAndVesselId", S(hash_key_of(&customer_id, &vessel_id)))
            .key("inventoryKey", S(sort_key_of(&inventory_type, &inventory_id)))
            .send()
            .instrument(self.instrumentation())
            .await?
            .item
            .map(from_item::<_, Inventory>)
            .map_or(Ok(None), |inventory| inventory.map(Some))
            .map_err(RuntimeError::from)
    }

    pub async fn delete_inventory(
        &self,
        customer_id: Uuid,
        vessel_id: Uuid,
        inventory_type: String,
        inventory_id: String,
    ) -> Result<(), RuntimeError> {
        self.client
            .delete_item()
            .table_name(self.table_name.as_str())
            .key("customerAndVesselId", S(hash_key_of(&customer_id, &vessel_id)))
            .key("inventoryKey", S(sort_key_of(&inventory_type, &inventory_id)))
            .send()
            .instrument(self.instrumentation())
            .await?;
        Ok(())
    }

    fn instrumentation(&self) -> Span {
        aws_metadata(
            self.client.conf().region().map(|value| value.to_string()).as_deref(),
            Some(self.table_name.as_str()),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::inventory_dao::{hash_key_of, sort_key_of};
    use crate::{Inventory, InventoryDao, RuntimeError};
    use async_trait::async_trait;
    use aws_config::load_from_env;
    use aws_sdk_dynamodb::config::Builder;
    use aws_sdk_dynamodb::operation::put_item::{PutItemError, PutItemOutput};
    use aws_sdk_dynamodb::types::{
        AttributeDefinition, AttributeValue::S, KeySchemaElement, KeyType, ProvisionedThroughput, ScalarAttributeType,
    };
    use aws_sdk_dynamodb::Client;
    use aws_smithy_http::result::SdkError;
    use chrono::{DateTime, FixedOffset, TimeZone, Utc};
    use std::collections::HashMap;
    use std::future::join;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use test_context::{test_context, AsyncTestContext};
    use tokio::test as tokio_test;
    use uuid::{uuid, Uuid};

    struct DynamoDbTestContext {
        client: Box<Client>,
        dao: Box<InventoryDao>,
        table_name: String,
    }

    static NUMBER: AtomicUsize = AtomicUsize::new(0);

    // customers
    static ID_0: Uuid = uuid!("00000000-0000-0000-0000-000000000000");
    // vessels
    static ID_1: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
    static ID_2: Uuid = uuid!("00000000-0000-0000-0000-000000000002");
    static ID_3: Uuid = uuid!("00000000-0000-0000-0000-000000000003");
    // inventory
    static INVENTORY_TYPE_0: &str = "pc";
    static INVENTORY_TYPE_1: &str = "radar";
    static INVENTORY_ID_0: &str = "012";
    static INVENTORY_ID_1: &str = "345";

    #[async_trait]
    impl AsyncTestContext for DynamoDbTestContext {
        async fn setup() -> DynamoDbTestContext {
            let table_name = format!("Inventory{}", NUMBER.fetch_add(1, Ordering::SeqCst));
            let config = load_from_env().await;
            let local_config = Builder::from(&config).endpoint_url("http://localhost:8000").build();
            let client = Client::from_conf(local_config);

            client
                .create_table()
                .table_name(table_name.as_str())
                .attribute_definitions(
                    AttributeDefinition::builder()
                        .attribute_name("customerAndVesselId")
                        .attribute_type(ScalarAttributeType::S)
                        .build(),
                )
                .attribute_definitions(
                    AttributeDefinition::builder()
                        .attribute_name("inventoryKey")
                        .attribute_type(ScalarAttributeType::S)
                        .build(),
                )
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("customerAndVesselId")
                        .key_type(KeyType::Hash)
                        .build(),
                )
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("inventoryKey")
                        .key_type(KeyType::Range)
                        .build(),
                )
                .provisioned_throughput(
                    ProvisionedThroughput::builder()
                        .read_capacity_units(1000)
                        .write_capacity_units(1000)
                        .build(),
                )
                .send()
                .await
                .unwrap();

            let context = DynamoDbTestContext {
                client: Box::new(client.clone()),
                dao: Box::new(InventoryDao::new(client, table_name.clone())),
                table_name: table_name.clone(),
            };

            let (res1, res2, res3) = join!(
                context.create_record(
                    &ID_0,
                    &ID_1,
                    INVENTORY_TYPE_0,
                    INVENTORY_ID_0,
                    Some("q1w2e3"),
                    None,
                    "2011-01-30T14:58:00+01:00"
                ),
                context.create_record(
                    &ID_0,
                    &ID_1,
                    INVENTORY_TYPE_0,
                    INVENTORY_ID_1,
                    None,
                    Some("im-12345"),
                    "2015-07-02T03:20:00+02:00"
                ),
                context.create_record(
                    &ID_0,
                    &ID_2,
                    INVENTORY_TYPE_1,
                    INVENTORY_ID_0,
                    Some("r@nd0m"),
                    None,
                    "2017-11-11T16:00:00+02:00"
                ),
            )
            .await;

            res1.unwrap();
            res2.unwrap();
            res3.unwrap();

            context
        }

        async fn teardown(self) {
            self.client
                .delete_table()
                .table_name(self.table_name)
                .send()
                .await
                .unwrap();
        }
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn create_inventory(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let created_at = Utc
            .with_ymd_and_hms(2015, 7, 2, 1, 20, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(7200).unwrap());

        let save = ctx
            .dao
            .create_inventory(Inventory {
                customer_id: ID_0,
                vessel_id: ID_2,
                inventory_type: INVENTORY_TYPE_1.to_string(),
                inventory_id: INVENTORY_ID_1.to_string(),
                serial_number: None,
                aws_instance_id: None,
                created_at,
            })
            .await;
        assert!(save.is_ok());

        let inventory = ctx
            .client
            .get_item()
            .table_name(ctx.table_name.as_str())
            .key("customerAndVesselId", S(hash_key_of(&ID_0, &ID_2)))
            .key(
                "inventoryKey",
                S(sort_key_of(&INVENTORY_TYPE_1.into(), &INVENTORY_ID_1.into())),
            )
            .send()
            .await?;
        assert!(inventory.item.is_some());
        assert_eq!(
            "2015-07-02T03:20:00+02:00",
            inventory.item.unwrap()["createdAt"].as_s().unwrap()
        );

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn get_inventory(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let created_at = Utc
            .with_ymd_and_hms(2011, 1, 30, 13, 58, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(3600).unwrap());

        let inventory = ctx
            .dao
            .get_inventory(ID_0, ID_1, INVENTORY_TYPE_0.into(), INVENTORY_ID_0.into())
            .await?
            .unwrap();
        assert!(inventory.aws_instance_id.is_none());
        assert_eq!(created_at, inventory.created_at);

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn get_inventory_unexisting(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx
            .dao
            .get_inventory(ID_0, ID_1, INVENTORY_TYPE_1.into(), INVENTORY_ID_1.into())
            .await?;
        assert!(unexisting.is_none());

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn delete_inventory(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let result = ctx
            .dao
            .delete_inventory(ID_0, ID_1, INVENTORY_TYPE_0.into(), INVENTORY_ID_0.into())
            .await;
        assert!(result.is_ok());

        let license = ctx
            .client
            .get_item()
            .table_name(ctx.table_name.as_str())
            .key("customerAndVesselId", S(hash_key_of(&ID_0, &ID_1)))
            .key(
                "inventoryKey",
                S(sort_key_of(&INVENTORY_TYPE_0.to_string(), &INVENTORY_ID_0.to_string())),
            )
            .send()
            .await?;
        assert!(license.item.is_none());

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn delete_inventory_unexisting(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx
            .dao
            .delete_inventory(ID_0, ID_1, INVENTORY_TYPE_1.into(), INVENTORY_ID_1.into())
            .await;
        assert!(unexisting.is_ok());

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn list_inventory(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx.dao.list_inventory(ID_0, ID_1, None).await;
        assert!(unexisting.is_ok());

        let results = unexisting.unwrap();
        assert_eq!(2, results.items.len());
        assert_eq!(INVENTORY_TYPE_0, results.items[0].inventory_type);
        assert_eq!(INVENTORY_ID_0, results.items[0].inventory_id);

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn list_inventory_page(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx
            .dao
            .list_inventory(
                ID_0,
                ID_1,
                Some(sort_key_of(&INVENTORY_TYPE_0.into(), &INVENTORY_ID_0.into())),
            )
            .await;
        assert!(unexisting.is_ok());

        let results = unexisting.unwrap();
        assert_eq!(1, results.items.len());
        assert_eq!(INVENTORY_TYPE_0, results.items[0].inventory_type);
        assert_eq!(INVENTORY_ID_1, results.items[0].inventory_id);

        Ok(())
    }

    #[test_context(DynamoDbTestContext)]
    #[tokio_test]
    async fn list_inventory_unexisting(ctx: &DynamoDbTestContext) -> Result<(), RuntimeError> {
        let unexisting = ctx.dao.list_inventory(ID_1, ID_2, None).await;
        assert!(unexisting.is_ok());

        let results = unexisting.unwrap();
        assert!(results.items.is_empty());
        assert!(results.last_evaluated_key.is_none());

        Ok(())
    }

    impl DynamoDbTestContext {
        async fn create_record(
            &self,
            customer_id: &Uuid,
            vessel_id: &Uuid,
            inventory_type: &str,
            inventory_id: &str,
            serial_number: Option<&str>,
            aws_instance_id: Option<&str>,
            created_at: &str,
        ) -> Result<PutItemOutput, SdkError<PutItemError>> {
            let mut request = self
                .client
                .put_item()
                .table_name(self.table_name.as_str())
                .item("customerAndVesselId", S(hash_key_of(customer_id, vessel_id)))
                .item(
                    "inventoryKey",
                    S(sort_key_of(&inventory_type.to_string(), &inventory_id.to_string())),
                )
                .item("customerId", S(customer_id.to_string()))
                .item("vesselId", S(vessel_id.to_string()))
                .item("inventoryType", S(inventory_type.into()))
                .item("inventoryId", S(inventory_id.into()))
                .item("createdAt", S(created_at.to_string()));

            if let Some(value) = serial_number {
                request = request.item("serialNumber", S(value.to_string()));
            }
            if let Some(value) = aws_instance_id {
                request = request.item("awsInstanceId", S(value.to_string()));
            }

            request.send().await
        }
    }
}

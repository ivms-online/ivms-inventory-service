/*
 * This file is part of the IVMS Online.
 *
 * @copyright 2023 © by Rafał Wrzeszcz - Wrzasq.pl.
 */

use env_logger::Builder;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::future::Future;
use tracing_core::dispatcher::set_global_default;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;
use xray::XRaySubscriber;

pub async fn run_lambda<PayloadType, HandlerType, FutureType, ReturnType, ErrorType>(
    func: HandlerType,
) -> Result<(), Error>
where
    PayloadType: for<'serde> Deserialize<'serde>,
    HandlerType: Fn(LambdaEvent<PayloadType>) -> FutureType,
    FutureType: Future<Output = Result<ReturnType, ErrorType>>,
    ReturnType: Serialize,
    ErrorType: Debug + Display,
{
    Builder::from_default_env().format_timestamp(None).init();

    set_global_default(Registry::default().with(XRaySubscriber::default()).into())?;

    run(service_fn(func)).await
}

#[macro_export]
macro_rules! run_lambda {
    ($handler:expr) => {
        run_lambda($handler).await
    };
}

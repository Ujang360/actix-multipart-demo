use actix_multipart::Multipart;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use env_logger::builder as log_builder;
use futures::{StreamExt, TryStreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::to_string as to_json;
use std::env::{set_var, var};
use std::io::Result as IOResult;
use std::net::SocketAddrV4;

const DEFAULT_BINDING_ADDRESS: &str = "0.0.0.0:5050";
const BINDING_ADDRESS: &str = "BINDING_ADDRESS";
const RUST_LOG: &str = "RUST_LOG";

#[derive(Debug, Deserialize, Serialize)]
struct ReceivedPart {
    content_type: String,
    content_disposition: Option<String>,
    content_data: Vec<u8>,
}

fn init_logger() {
    if var(RUST_LOG).is_err() {
        #[cfg(debug_assertions)]
        set_var(RUST_LOG, "debug,actix_server=debug,actix_web=debug");
        #[cfg(not(debug_assertions))]
        set_var(RUST_LOG, "info,actix_server=info,actix_web=info");
    }

    log_builder().default_format().format_timestamp_nanos().format_indent(Some(2)).init();
}

fn load_binding_address() -> String {
    dotenv().ok();

    match var(BINDING_ADDRESS) {
        Err(_) => DEFAULT_BINDING_ADDRESS.to_string(),
        Ok(env_binding_address) => {
            if env_binding_address.parse::<SocketAddrV4>().is_err() {
                error!(
                    "Invalid SockedAddrV4 => \"{}\", using the default \"{}\"",
                    env_binding_address, DEFAULT_BINDING_ADDRESS
                );
                DEFAULT_BINDING_ADDRESS.to_string()
            } else {
                env_binding_address
            }
        }
    }
}

async fn receive_multiparts(mut payload: Multipart) -> impl Responder {
    let mut received_parts = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_type().to_string();
        let content_disposition = if let Some(fcd) = field.content_disposition() {
            Some(format!("{:#?}", fcd))
        } else {
            None
        };
        let mut content_data = Vec::new();

        while let Some(Ok(chunk)) = field.next().await {
            content_data.extend(chunk);
        }

        received_parts.push(ReceivedPart { content_data, content_type, content_disposition });
    }

    info!("Got {:?}", received_parts);
    let received_parts_as_json = to_json(&received_parts).unwrap();

    HttpResponse::Ok().body(received_parts_as_json)
}

#[actix_web::main]
async fn main() -> IOResult<()> {
    init_logger();
    let binding_address = load_binding_address();
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(web::resource("/").route(web::post().to(receive_multiparts)))
    })
    .bind(&binding_address)?
    .run()
    .await
}

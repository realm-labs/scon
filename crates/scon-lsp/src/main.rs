#![allow(deprecated)]

mod completion;
mod config;
mod definition;
mod diagnostics;
mod formatting;
mod hover;
mod position;
mod server;
mod state;
mod symbols;

#[tokio::main]
async fn main() {
    server::run().await;
}

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix_web::{web::Data, App, HttpServer};
use channel::Channel;
use client::Client;
use mediasoup::worker_manager::WorkerManager;

mod channel;
mod client;
mod handlers;
mod options;
mod util;

pub type MutexMap<T> = Mutex<HashMap<String, T>>;
pub type Clients = MutexMap<Arc<Client>>;
pub type Channels = MutexMap<Arc<Channel>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let worker_manager = Data::new(WorkerManager::new());
    let clients: Data<Clients> = Data::new(Mutex::new(HashMap::new()));
    let channels: Data<Channels> = Data::new(Mutex::new(HashMap::new()));

    HttpServer::new(move || {
        App::new()
            .app_data(worker_manager.clone())
            .app_data(Data::clone(&clients))
            .app_data(Data::clone(&channels))
            .service(handlers::join_vc)
            .service(handlers::create_c2s_transport)
    })
    .workers(2)
    .bind("127.0.0.1:3000")?
    .run()
    .await
}

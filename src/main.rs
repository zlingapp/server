use std::{collections::HashMap, sync::Mutex};

use actix_web::{web::Data, App, HttpServer};
use log::info;
use mediasoup::worker_manager::WorkerManager;
use voice::{VoiceChannels, VoiceClients};

mod util;
mod voice;

pub type MutexMap<T> = Mutex<HashMap<String, T>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // voice chat related
    voice::options::initialize_all();
    voice::options::print_all();

    let voice_worker_manager = Data::new(WorkerManager::new());
    let voice_clients: Data<VoiceClients> = Data::new(Mutex::new(HashMap::new()));
    let voice_channels: Data<VoiceChannels> = Data::new(Mutex::new(HashMap::new()));

    HttpServer::new(move || {
        // add logging middleware
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            // setup voice api
            .app_data(voice_worker_manager.clone())
            .app_data(Data::clone(&voice_clients))
            .app_data(Data::clone(&voice_channels))
            .service(voice::handlers::scope())
    })
    .workers(2)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

use std::{collections::HashMap, env, sync::Mutex};

use actix_web::{
    web::{self, Data},
    App, HttpServer,
};
use db::Database;
use log::{error, info};
use mediasoup::worker_manager::WorkerManager;
use sqlx::postgres::PgPoolOptions;
use voice::{VoiceChannels, VoiceClients};

use crate::{db::DB, realtime::pubsub::consumer_manager::EventConsumerManager};

mod auth;
mod channels;
mod crypto;
mod db;
mod guilds;
mod media;
mod messaging;
mod options;
mod realtime;
mod security;
mod settings;
mod util;
mod voice;

pub type MutexMap<T> = Mutex<HashMap<String, T>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // override RUST_LOG if it's not set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info,sqlx::query=warn")
    }

    // initialize logger
    env_logger::init();

    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    options::initialize_all();
    options::print_all();

    let db_url = options::db_conn_string();

    // database
    info!("Connecting to database...");
    let pool = PgPoolOptions::new().max_connections(5).connect(&db_url);

    let pool = match pool.await {
        Ok(pool) => {
            info!("Connected to database successfully!");
            pool
        }
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    let pool: DB = Data::new(Database::with_pool(pool));

    // voice chat related
    let voice_worker_manager = Data::new(WorkerManager::new());
    let voice_clients: Data<VoiceClients> = Data::new(Mutex::new(HashMap::new()));
    let voice_channels: Data<VoiceChannels> = Data::new(Mutex::new(HashMap::new()));

    // pubsub
    let event_manager = Data::new(EventConsumerManager::new());

    HttpServer::new(move || {
        // add logging middleware
        App::new()
            // logging
            .wrap(actix_web::middleware::Logger::new("%{r}a %r -> %s in %Dms").log_target("http"))
            // database
            .app_data(Data::clone(&pool))
            // authentication
            .configure(auth::routes::configure_app)
            // voice chat
            .app_data(Data::clone(&voice_worker_manager))
            .app_data(Data::clone(&voice_clients))
            .app_data(Data::clone(&voice_channels))
            .configure(voice::routes::configure_app)
            // guilds
            .configure(guilds::routes::configure_app)
            // channels
            .configure(channels::routes::configure_app)
            // pubsub
            .app_data(Data::clone(&event_manager))
            .service(realtime::pubsub::events::events_ws)
            // messaging
            .configure(messaging::routes::configure_app)
            // file uploads
            .configure(media::routes::configure_app)
            .configure(settings::routes::configure_app)
            .default_service(web::route().to(api_endpoint_not_found))
    })
    .workers(2)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn api_endpoint_not_found() -> actix_web::HttpResponse {
    actix_web::HttpResponse::NotFound().body("api_endpoint_not_found")
}

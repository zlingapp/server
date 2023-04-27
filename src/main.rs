use std::{collections::HashMap, sync::Mutex};

use actix_web::{web::Data, App, HttpServer};
use log::{error, info};
use mediasoup::worker_manager::WorkerManager;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use voice::{VoiceChannels, VoiceClients};

use crate::auth::user::UserManager;

mod auth;
mod options;
mod util;
mod voice;
mod guilds;
mod channels;

pub type MutexMap<T> = Mutex<HashMap<String, T>>;
pub type DB = Data<Pool<Postgres>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    options::initialize_all();
    options::print_all();

    let db_url = options::db_conn_string();

    // database
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

    let pool: DB = Data::new(pool);

    // auth related
    let user_manager = Data::new(UserManager::new(Data::clone(&pool)));

    // voice chat related
    let voice_worker_manager = Data::new(WorkerManager::new());
    let voice_clients: Data<VoiceClients> = Data::new(Mutex::new(HashMap::new()));
    let voice_channels: Data<VoiceChannels> = Data::new(Mutex::new(HashMap::new()));

    HttpServer::new(move || {
        // add logging middleware
        App::new()
            .wrap(actix_web::middleware::Logger::new("%{r}a %r -> %s in %Dms").log_target("http"))
            .app_data(Data::clone(&pool))
            // setup voice api
            .app_data(Data::clone(&voice_worker_manager))
            .app_data(Data::clone(&voice_clients))
            .app_data(Data::clone(&voice_channels))
            .service(voice::handlers::scope())
            .app_data(Data::clone(&user_manager))
            .service(auth::handlers::scope())
            .service(guilds::handlers::scope())
            .service(channels::handlers::scope())
    })
    .workers(2)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

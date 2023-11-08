use std::{collections::HashMap, env, sync::Mutex};

use actix_cors::Cors;
use actix_web::{
    middleware::Condition,
    web::{self, Data},
    App, HttpServer,
};
use db::Database;
use log::{error, info, warn};
use mediasoup::worker_manager::WorkerManager;
use sqlx::{migrate, postgres::PgPoolOptions};
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use voice::{VoiceChannels, VoiceClients};

use crate::{db::DB, realtime::pubsub::pubsub::PubSub, voice::pool::VoiceWorkerPool};

mod apidocs;
mod auth;
mod bot;
mod channels;
mod crypto;
mod db;
mod error;
mod friends;
mod guilds;
mod media;
mod messaging;
mod options;
mod realtime;
mod security;
mod settings;
mod util;
mod voice;

// shortcut to make a Mutexed String to T hashmap
pub type MutexMap<T> = Mutex<HashMap<String, T>>;

#[derive(OpenApi)]
#[openapi()]
struct ApiDoc;

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

    if *options::DB_RUN_MIGRATIONS {
        info!("Running database migrations...");

        migrate!()
            .run(&pool)
            .await
            .map_err(|e| {
                error!("Failed to run migrations: {}", e);
                std::process::exit(1);
            })
            .unwrap();
    } else {
        warn!("Database migrations will not be run. Inconsistent database state may occur!")
    }

    let pool: DB = Data::new(Database::with_pool(pool));

    // voice chat related
    let worker_manager = WorkerManager::new();
    let voice_ports = options::voice_ports();
    let voice_worker_pool = Data::new(Mutex::new(VoiceWorkerPool::new(
        worker_manager,
        voice_ports,
    )));
    let voice_clients: Data<VoiceClients> = Data::new(Mutex::new(HashMap::new()));
    let voice_channels: Data<VoiceChannels> = Data::new(Mutex::new(HashMap::new()));

    // pubsub
    let event_manager = Data::new(PubSub::new());

    let mut server = HttpServer::new(move || {
        let oapi = apidocs::setup_oapi();

        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            // logging
            .wrap(actix_web::middleware::Logger::new("%{r}a %r -> %s in %Dms").log_target("http"))
            // cors acess
            .wrap(Condition::new(*options::HANDLE_CORS, cors))
            // database
            .app_data(Data::clone(&pool))
            // authentication
            .configure(auth::routes::configure_app)
            // voice chat
            .app_data(Data::clone(&voice_worker_pool))
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
            // bots
            .configure(bot::routes::configure_app)
            // friends
            .configure(friends::routes::configure_app)
            .default_service(web::route().to(api_endpoint_not_found))
            // OpenAPI docs
            .service(
                RapiDoc::with_openapi("/openapi.json", oapi)
                    .custom_html(include_str!("../res/rapidoc.html"))
                    .path("/docs"),
            )
    })
    .workers(*options::NUM_WEB_WORKERS);

    if !*options::SSL_ONLY {
        info!(
            "Starting HTTP server on {}:{}",
            options::BIND_ADDR.ip(),
            options::BIND_ADDR.port()
        );
        server = server.bind(options::bind_addr())?;
    }

    if *options::SSL_ENABLE {
        info!(
            "Starting HTTPS server on {}:{} (via rustls)",
            options::SSL_BIND_ADDR.ip(),
            options::SSL_BIND_ADDR.port()
        );
        server = server.bind_rustls_021(options::ssl_bind_addr(), options::ssl_config())?;
    }

    if !*options::HANDLE_CORS {
        info!("CORS will not be handled: expecting a reverse proxy in front of this server");
    }

    server.run().await
}

async fn api_endpoint_not_found() -> actix_web::HttpResponse {
    actix_web::HttpResponse::NotFound()
        .content_type("text/html")
        .body(
            r#"
            <h2>404 Not Found</h2>
            <h5>Zling API</h5>
            <p>The requested API endpoint was not found.</p>
            <a href="/docs">View API Documentation</a>
            <style>
                body {
                    font-family: sans-serif;
                    text-align: center;
                }
            </style>
        "#,
        )
}

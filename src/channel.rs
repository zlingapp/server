use std::sync::{Arc, Mutex};

use crate::{
    client::Client,
    options::{router_options, worker_settings},
    Channels,
};
use actix_web::web::Data;
use log::debug;
use mediasoup::{router::Router, worker::Worker, worker_manager::WorkerManager};

#[derive(Debug)]
pub struct Channel {
    pub id: String,
    pub clients: Mutex<Vec<Arc<Client>>>,
    pub router: Router,
    pub worker: Worker,
}

impl Channel {
    pub async fn new_with_id(id: String, wm: Arc<WorkerManager>) -> Self {
        let worker = wm.create_worker(worker_settings()).await.unwrap();
        let router = worker.create_router(router_options()).await.unwrap();

        Self {
            id: id.to_owned(),
            clients: Mutex::new(Vec::new()),
            router,
            worker,
        }
    }
}

pub async fn create_channel(id: &str, channels: Data<Channels>, wm: Arc<WorkerManager>) -> Arc<Channel> {
    debug!("creating channel...");
    let channel = Channel::new_with_id(id.to_owned(), wm).await;
    let channel = Arc::new(channel);

    let mut channels = channels.lock().unwrap();
    channels.insert(channel.id.clone(), channel.clone());

    return channel;
}

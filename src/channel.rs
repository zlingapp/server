use std::{sync::{Arc, Mutex}, num::NonZeroU16};

use crate::{
    client::Client,
    options::{router_options, worker_settings},
    Channels, Clients,
};
use actix_web::web::Data;
use log::{debug, info, warn};
use mediasoup::{router::Router, worker::Worker, worker_manager::WorkerManager, prelude::{AudioLevelObserver, AudioLevelObserverOptions}};

pub struct Channel {
    pub id: String,
    pub clients: Mutex<Vec<Arc<Client>>>,
    pub router: Router,
    pub worker: Worker,
    pub al_observer: AudioLevelObserver,
}

impl Channel {
    pub async fn new_with_id(id: String, wm: Arc<WorkerManager>) -> Self {
        let worker = wm.create_worker(worker_settings()).await.unwrap();
        let router = worker.create_router(router_options()).await.unwrap();

        let al_observer = router
            .create_audio_level_observer({
                let mut options = AudioLevelObserverOptions::default();
                options.max_entries = NonZeroU16::new(1).unwrap();
                options.threshold = -70;
                options.interval = 1;
                options
            })
            .await
            .unwrap();

        al_observer.on_volumes(|volumes| {
            for volume in volumes {
                info!("Volume of {}: {} dB", volume.producer.id(), volume.volume);
            }
        }).detach();

        Self {
            id: id.to_owned(),
            clients: Mutex::new(Vec::new()),
            router,
            worker,
            al_observer,
        }
    }

    pub async fn erase_client(&self, client_identity: &str, global_clients: Arc<Clients>, global_channels: Arc<Channels>) {
        info!("client {:?} has disconnected", client_identity);
        self.clients.lock().unwrap().retain(|c| c.identity != client_identity);
        let removed = global_clients.lock().unwrap().remove(client_identity);
        if removed.is_none() {
            // this should never happen in theory
            warn!("Channel::erase_client() called twice for client {:?}", client_identity);
        }
        
        if self.clients.lock().unwrap().is_empty() {
            // at this point the only reference to this channel is the one in the channels map
            // so we can safely remove it from the map
            global_channels.lock().unwrap().remove(&self.id);
        }
    }

    pub async fn disconnect_client(&self, client: &Client, global_clients: Arc<Clients>, global_channels: Arc<Channels>) {
        self.erase_client(&client.identity, global_clients, global_channels).await;
        self.notify_client_left(client).await;
    }

    pub async fn destroy(&self, global_clients: Arc<Clients>, global_channels: Arc<Channels>) {
        for client in self.clients.lock().unwrap().iter() {
            self.disconnect_client(client, global_clients.clone(), global_channels.clone()).await;
            // on the last iteration, the channel will be removed from the channels map, destroying it
        }
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        info!("dropping channel {:?}", self.id);
    }
}

pub async fn create_channel(
    id: &str,
    channels: Data<Channels>,
    wm: Arc<WorkerManager>,
) -> Arc<Channel> {
    debug!("creating channel...");
    let channel = Channel::new_with_id(id.to_owned(), wm).await;
    let channel = Arc::new(channel);

    let mut channels = channels.lock().unwrap();
    channels.insert(channel.id.clone(), channel.clone());

    return channel;
}

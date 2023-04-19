use std::{
    num::NonZeroU16,
    sync::{Arc, Mutex},
};

use crate::voice::{
    client::VoiceClient,
    options::{router_options, worker_settings},
    VoiceChannels, VoiceClients,
};
use actix_web::web::Data;
use log::{info, warn};
use mediasoup::{
    prelude::{AudioLevelObserver, AudioLevelObserverOptions},
    router::Router,
    worker::Worker,
    worker_manager::WorkerManager,
};

pub struct VoiceChannel {
    pub id: String,
    pub clients: Mutex<Vec<Arc<VoiceClient>>>,
    pub router: Router,
    pub worker: Worker,
    pub al_observer: AudioLevelObserver,
}

impl VoiceChannel {
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

        // al_observer.on_volumes(|volumes| {
        //     for volume in volumes {
        //         info!("Volume of {}: {} dB", volume.producer.id(), volume.volume);
        //     }
        // }).detach();

        Self {
            id: id.to_owned(),
            clients: Mutex::new(Vec::new()),
            router,
            worker,
            al_observer,
        }
    }

    pub async fn erase_client(
        &self,
        client_identity: &str,
        global_clients: Arc<VoiceClients>,
        global_channels: Arc<VoiceChannels>,
    ) {
        
        // remove the client from the channel
        self.clients
            .lock()
            .unwrap()
            .retain(|c| c.identity != client_identity);

        // remove the client from the global clients map
        let removed = global_clients.lock().unwrap().remove(client_identity);
        if removed.is_none() {
            // this should never happen in theory
            warn!(
                "client[{:?}]: Channel::erase_client() called twice",
                client_identity
            );
        } else {
            removed.unwrap().cleanup().await;
        }

        // if the channel is empty, remove it from the global channels map
        if self.clients.lock().unwrap().is_empty() {
            // at this point the only reference to this channel is the one in the channels map
            // so we can safely remove it from the map
            global_channels.lock().unwrap().remove(&self.id);
        }

        info!(
            "client[{:?}]: disconnected from {:?}, remaining: {}",
            client_identity,
            self.id,
            self.clients.lock().unwrap().len()
        );
    }

    pub async fn disconnect_client(
        &self,
        client: &VoiceClient,
        global_clients: Arc<VoiceClients>,
        global_channels: Arc<VoiceChannels>,
    ) {
        self.erase_client(&client.identity, global_clients, global_channels)
            .await;
        self.notify_client_left(client).await;
    }
}

impl Drop for VoiceChannel {
    fn drop(&mut self) {
        info!("channel[{:?}]: dropped", self.id);
    }
}

pub async fn create_channel(
    id: &str,
    channels: Data<VoiceChannels>,
    wm: Arc<WorkerManager>,
) -> Arc<VoiceChannel> {
    let channel = VoiceChannel::new_with_id(id.to_owned(), wm).await;
    let channel = Arc::new(channel);

    let mut channels = channels.lock().unwrap();
    channels.insert(channel.id.clone(), channel.clone());

    info!("channel[{:?}]: created", channel.id);

    return channel;
}

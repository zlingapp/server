use std::{sync::{Arc, Mutex}, num::NonZeroU16};

use crate::{
    client::Client,
    options::{router_options, worker_settings},
    Channels,
};
use actix_web::web::Data;
use log::{debug, info};
use mediasoup::{router::Router, worker::Worker, worker_manager::WorkerManager, prelude::{AudioLevelObserver, AudioLevelObserverOptions}};

#[derive(Debug)]
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

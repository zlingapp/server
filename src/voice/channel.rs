use std::{num::NonZeroU16, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    options,
    voice::{client::VoiceClient, VoiceChannels, VoiceClients},
};
use actix_web::web::Data;
use log::{info, warn};
use mediasoup::{
    prelude::{AudioLevelObserver, AudioLevelObserverOptions},
    router::Router,
    webrtc_server::WebRtcServer,
    webrtc_transport::WebRtcTransport,
    worker::RequestError,
};

use super::pool::VoiceWorkerPool;

pub struct VoiceChannel {
    pub id: String,
    pub clients: Mutex<Vec<Arc<VoiceClient>>>,
    pub router: Router,
    pub webrtc_server: WebRtcServer,
    pub al_observer: AudioLevelObserver,
}

impl VoiceChannel {
    pub async fn new_with_id(id: String, vwp: &Mutex<VoiceWorkerPool>) -> Self {
        // TODO: do not unwrap this
        let (router, webrtc_server) = { vwp.lock().await.allocate_router().await.unwrap() };

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
            webrtc_server,
            al_observer,
        }
    }

    pub async fn erase_client(
        &self,
        client_identity: &str,
        global_clients: &VoiceClients,
        global_channels: &VoiceChannels,
    ) {
        info!("Channel::erase_client()");

        // remove the client from the channel
        self.clients
            .lock()
            .await
            .retain(|c| c.identity != client_identity);

        // remove the client from the global clients map
        let removed = global_clients.lock().unwrap().remove(client_identity);

        match removed {
            Some(r) => r.cleanup(),
            None => {
                // this should never happen in theory
                warn!(
                    "client[{:?}]: Channel::erase_client() called twice",
                    client_identity
                )
            }
        }

        // if the channel is empty, remove it from the global channels map
        if self.clients.lock().await.is_empty() {
            // at this point the only reference to this channel is the one in the channels map
            // so we can safely remove it from the map
            global_channels.lock().unwrap().remove(&self.id);
        }

        println!("{:?}", global_channels.lock().unwrap().len());

        info!(
            "client[{:?}]: disconnected from {:?}, remaining: {}",
            client_identity,
            self.id,
            self.clients.lock().await.len()
        );
    }

    pub async fn disconnect_client(
        &self,
        client: &VoiceClient,
        global_clients: &VoiceClients,
        global_channels: &VoiceChannels,
    ) {
        self.erase_client(&client.identity, global_clients, global_channels)
            .await;
        self.notify_client_left(client).await;
    }

    pub async fn create_webrtc_transport(&self) -> Result<WebRtcTransport, RequestError> {
        self.router
            .create_webrtc_transport(options::webrtc_transport_options(
                self.webrtc_server.clone(),
            ))
            .await
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
    vwp: &Mutex<VoiceWorkerPool>,
) -> Arc<VoiceChannel> {
    let channel = VoiceChannel::new_with_id(id.to_owned(), vwp).await;
    let channel = Arc::new(channel);

    let mut channels = channels.lock().unwrap();
    channels.insert(channel.id.clone(), channel.clone());

    info!("channel[{:?}]: created", channel.id);

    channel
}

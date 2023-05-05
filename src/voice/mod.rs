use std::sync::Arc;

use crate::MutexMap;

use self::{channel::VoiceChannel, client::VoiceClient};

pub mod channel;
pub mod client;
pub mod routes;
pub mod transport;

pub type VoiceClients = MutexMap<Arc<VoiceClient>>;
pub type VoiceChannels = MutexMap<Arc<VoiceChannel>>;

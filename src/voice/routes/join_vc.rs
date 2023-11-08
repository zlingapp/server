use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use actix_rt::time::sleep;
use actix_web::{
    get,
    web::{Data, Json, Query},
};

use log::{info, warn};
use mediasoup::rtp_parameters::RtpCapabilitiesFinalized;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    auth::user::User,
    voice::{
        channel::create_channel, client::VoiceClient, pool::VoiceWorkerPool, VoiceChannels,
        VoiceClients,
    },
};

#[derive(Deserialize, IntoParams)]
pub struct JoinVcQuery {
    /// Channel ID - will be created if it does not already exist
    #[serde(rename = "c")]
    channel_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JoinVcReply {
    /// Supply as `RTC-Identity` header for any subsequent Voice API requests please
    identity: String,
    /// Supply as `RTC-Token` header for any subsequent Voice API requests please
    token: String,
    /// Router RTP capabilities, feed to `device.load` on your client.
    #[schema(value_type = Object)]
    rtp: RtpCapabilitiesFinalized,
}

// #[derive(Debug, Display, Error)]
// pub struct JoinVcError {}

// impl error::ResponseError for JoinVcError {
//     fn status_code(&self) -> actix_web::http::StatusCode {
//         actix_web::http::StatusCode::CONFLICT
//     }
// }

// pub type JoinVcResponse = Result<Json<JoinVcReply>, JoinVcError>;

/// Join voice channel
///
/// This should be the first API call you make when you want to establish a
/// voice connection. This endpoint gives you back the information you need to
/// connect to the voice websocket and create the correct mediasoup objects in
/// order to send and receive audio.
///
/// You need to connect to the voice websocket within 10 seconds of calling this
/// endpoint, otherwise the voice connection will be dropped and your
/// credentials will be invalidated. Peers already in the voice channel will not
/// be notified of your connection until you connect to the websocket.
///
/// Note that if a channel with the given ID does not exist, it will be created.
///
/// ### Connection Process
/// ```
/// 1. GET /voice/join?c=1234
///    -> identity, token, routerRtpCapabilities (as "rtp" field in response)
///
/// 2. Connect WS to /voice/ws (using identity & token)
/// ```
///
/// You are now considered "connected" to the voice channel, though you are not
/// yet sending or receiving any video or audio. To begin sending voice data, in
/// your mediasoup client library of choice:
///
/// ```js
/// device = new mediasoup.Device();
/// await device.load({ routerRtpCapabilities });
///
/// // set `RTC-Identity` and `RTC-Token` headers from now on!
/// // tell the server to create a server-side send transport
/// send_transport_parameters = http_post("/voice/transport/create?type=send");
/// // make it locally
/// send_transport = device.createSendTransport(send_transport_parameters);
///
/// // request a producer from your local send transport, this makes mediasoup automatically
/// // start the connection process with the server-side transport
/// producer = await send_transport.produce({ track: audio_track });
///
/// // your send_transport will now fire a `connect` event when it wants you to connect to the server
/// function handle_connect_event({ dtlsParameters }, callback) {
///     // tell the server to let you connect the send transport
///     http_post("/voice/transport/connect?type=send", { dtlsParameters });
///     // let mediasoup know it worked
///     callback();
/// }
///
/// // then, it will fire a `produce` event when it wants you to create a server-side producer
/// function handle_produce_event({ kind, rtpParameters }, callback) {
///     // make a server-side producer using an API call
///     { id } = http_post("/voice/produce", { kind, rtpParameters });
///     // let mediasoup know it worked
///     callback({ id });
/// }
///
/// // ...and you're live!
/// ```
/// Note that the above is pseudocode and may not necessarily reflect working
/// code. If in doubt, check out [how the Zling web app does
/// it](https://github.com/zlingapp/zvelte/blob/903fbe7e5c08dbc1279753aa260790e0bf5c23c8/src/components/voice/VoiceManager.svelte#L248).
#[utoipa::path(
    tag = "voice",
    security(("token" = [])),
    params(JoinVcQuery),
    responses(
        (status = OK, description = "Ready for websocket connection", body = JoinVcReply),
    )
)]
#[get("/voice/join")]
pub async fn join_vc(
    user: User,
    clients: Data<VoiceClients>,
    channels: Data<VoiceChannels>,
    vwp: Data<Mutex<VoiceWorkerPool>>,
    query: Query<JoinVcQuery>,
) -> Json<JoinVcReply> {
    // get the channel
    let channel = channels.lock().unwrap().get(&query.channel_id).cloned();
    // channels lock is released here

    let channel = match channel {
        // existing channel exists
        Some(existing) => existing.clone(),
        // create a new channel
        None => create_channel(&query.channel_id, channels.clone(), &vwp.into_inner()).await,
    };

    // create a new client
    let client = VoiceClient::with_channel_and_user(channel.clone(), user.into());
    let client = Arc::new(client);

    // add the client to the channel's client list
    channel.clients.lock().unwrap().push(client.clone());

    // create the reply
    let reply = JoinVcReply {
        identity: client.identity.clone(),
        token: client.token.clone(),
        rtp: channel.router.rtp_capabilities().clone(),
    };

    // the following closure checks if the client has connected to the websocket in time
    // if it hasn't, the client is removed from the channel silently
    {
        // two clones of the Arc<Client> are created here
        let client_inner = client.clone();
        let client_outer = client.clone();

        let clients_inner = clients.clone();

        let handle = Some(actix_rt::spawn(async move {
            sleep(Duration::from_secs(10)).await;

            // if the client hasn't connected to the websocket yet, remove it from the channel

            if !match client_inner.socket.read().unwrap().as_ref() {
                Some(socket) => socket.is_connected().await,
                None => false,
            } {
                warn!(
                    "client[{:?}]: initial connect: didn't connect to the websocket in time, removing",
                    client_inner.identity
                );

                client_inner
                    .channel
                    .erase_client(&client_inner.identity, &clients_inner, &channels)
                    .await;
            }
        }));

        *client_outer
            .socket_initial_connect_watch_handle
            .lock()
            .unwrap() = handle;
    }

    info!(
        "client[{:?}]: joined channel {:?}",
        client.identity, channel.id
    );

    // add the client to the global client list
    clients
        .lock()
        .unwrap()
        .insert(client.identity.clone(), client);

    Json(reply)
}

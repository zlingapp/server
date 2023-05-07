use std::{
    sync::{Arc, Mutex, RwLock, Weak},
    time::Duration, hash::{Hash, Hasher},
};

use actix_rt::{task::JoinHandle, time::sleep};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::{Message, MessageStream, Session};
use futures::StreamExt;

pub type Callback<T> = Box<dyn Fn(T) -> () + Send + Sync>;

#[derive(Debug)]
pub enum DisconnectReason {
    /// Called when the client disconnects because they didn't send a heartbeat in time.
    /// This usually means the client is no longer connected to the internet.
    PingTimeout,
    /// Called when the stream of messages from the client ends.
    /// This usually means voluntary disconnect.
    ReadExaust,
}

#[derive(Debug)]
pub enum SendFailureReason {
    SessionClosed,
    NoSession,
}

pub struct Socket {
    /// Internal nanoid, randomly generated.
    pub id: String,
    
    session: RwLock<Option<actix_ws::Session>>,
    watchdog_handle: Mutex<Option<JoinHandle<()>>>,
    /// The last time we received a ping from the client.
    pub last_ping: RwLock<Option<std::time::Instant>>,
    // callbacks
    /// Called when the client sends a message to the server.
    pub on_message: Option<Callback<String>>,
    /// Called when the client disconnects, for any reason.
    pub on_disconnect: Option<Callback<DisconnectReason>>,
}

impl Socket {
    /// Returns true if the socket is connected.
    /// Locks: session(write)
    pub async fn is_connected(&self) -> bool {
        if let Some(session) = self.session.write().unwrap().as_mut() {
            return session.ping(b"").await.is_ok();
        }
        false
    }

    fn internal_on_message(&self, msg: String) {
        if msg == "heartbeat" {
            *self.last_ping.write().unwrap() = Some(std::time::Instant::now());
            return;
        }

        if let Some(on_message) = &self.on_message {
            on_message(msg);
        }
    }

    fn internal_on_disconnect(&self, reason: DisconnectReason) {
        if let Some(on_disconnect) = &self.on_disconnect {
            on_disconnect(reason);
        }
    }

    fn spawn_read_loop(weak: Weak<Socket>, mut session: Session, mut msg_stream: MessageStream) {
        actix_rt::spawn(async move {
            while let Some(Ok(msg)) = msg_stream.next().await {
                let socket = match weak.upgrade() {
                    Some(strong) => strong,
                    None => return,
                };
                match msg {
                    Message::Ping(bytes) => {
                        if session.pong(&bytes).await.is_err() {
                            return;
                        }
                    }
                    Message::Text(s) => {
                        Socket::internal_on_message(&socket, s.to_string());
                    }
                    Message::Close(_) => break,
                    _ => {}
                };
            }

            let socket = match weak.upgrade() {
                Some(strong) => strong,
                None => return,
            };

            // disconnect
            if let Some(session) = socket.session.write().unwrap().take() {
                session.close(None).await.unwrap_or(());
            }

            socket.internal_on_disconnect(DisconnectReason::ReadExaust);
        });
    }

    fn spawn_heartbeat_watchdog(weak: Weak<Socket>, interval: Duration) -> JoinHandle<()> {
        actix_rt::spawn(async move {
            loop {
                sleep(interval).await;

                let socket = match weak.upgrade() {
                    Some(strong) => strong,
                    None => return,
                };

                // if the client hasn't sent a heartbeat in 10 seconds, remove it from the channel
                let last_ping = socket.last_ping.read().unwrap();

                if last_ping.is_none() || last_ping.unwrap().elapsed().as_secs() > 10 {
                    if let Some(session) = socket.session.write().unwrap().take() {
                        session.close(None).await.unwrap_or(());
                    }

                    socket.internal_on_disconnect(DisconnectReason::PingTimeout);
                    return;
                }
            }
        })
    }

    /// Returns an instance of Socket and the response to send to the client.
    pub fn new_arc_from_request(
        // this should be random
        socket_id: String,
        req: &HttpRequest,
        body: web::Payload,
        on_message: Option<Callback<String>>,
        on_disconnect: Option<Callback<DisconnectReason>>,
    ) -> Result<(Arc<Self>, HttpResponse), actix_web::Error> {
        let (response, session, msg_stream) = actix_ws::handle(&req, body)?;

        let instance = Arc::new(Self {
            id: socket_id,
            session: RwLock::new(Some(session.clone())),
            last_ping: RwLock::new(None),
            watchdog_handle: Mutex::new(None),
            on_message,
            on_disconnect,
        });

        if instance.watchdog_handle.lock().unwrap().is_some() {
            panic!("watchdog already spawned");
        }

        let watchdog_handle =
            Socket::spawn_heartbeat_watchdog(Arc::downgrade(&instance), Duration::from_secs(10));
        *instance.watchdog_handle.lock().unwrap() = Some(watchdog_handle);

        Socket::spawn_read_loop(Arc::downgrade(&instance), session, msg_stream);

        Ok((instance, response))
    }

    pub async fn send(&self, msg: String) -> Result<(), SendFailureReason> {
        use SendFailureReason::*;

        if let Some(session) = self.session.write().unwrap().as_mut() {
            session.text(msg).await.map_err(|_| SessionClosed)
        } else {
            Err(NoSession)
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        if let Some(watchdog) = self.watchdog_handle.lock().unwrap().take() {
            // we connected successfully, so stop the watchdog now
            watchdog.abort();
        }
        if let Some(session) = self.session.write().unwrap().take() {
            // we connected successfully, so close the session now
            actix_rt::spawn(async move {
                session.close(None).await.unwrap_or(());
            });
        }
    }
}

impl Hash for Socket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Socket {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Socket {}
use std::io;

use mediasoup::{
    router::Router, webrtc_server::WebRtcServer, worker::Worker, worker_manager::WorkerManager,
};

use crate::options;

pub struct VoiceWorkerPool {
    worker_manager: WorkerManager,
    workers: Vec<(Worker, WebRtcServer)>,
    // must be a valid index into workers
    next_worker: usize,
    // must have at least 1 port
    // this will get popped from the end, so preferably put lower ports last
    ports: Vec<u16>,
}
impl VoiceWorkerPool {
    pub fn new(worker_manager: WorkerManager, ports: Vec<u16>) -> Self {
        Self {
            worker_manager,
            workers: vec![],
            next_worker: 0,
            ports,
        }
    }

    pub async fn allocate_router(&mut self) -> io::Result<(Router, WebRtcServer)> {
        let worker;
        let webrtc_server;

        if self.workers.len() < self.ports.len() {
            // there are still unallocated ports
            (worker, webrtc_server) = self.create_worker_and_server().await?;
        } else {
            (worker, webrtc_server) = self.get_existing_worker_and_server();
        };

        let router = worker
            .create_router(options::router_options())
            .await
            .unwrap();

        Ok((router, webrtc_server))
    }
    async fn create_worker_and_server(&mut self) -> io::Result<(&Worker, WebRtcServer)> {
        let worker = self
            .worker_manager
            .create_worker(options::worker_settings())
            .await?;

        // create a new webrtc server
        let port = self.ports.pop().unwrap();
        let opts = options::webrtc_server_options(port);
        let server = worker.create_webrtc_server(opts).await.unwrap(); // todo: handle error here possibly

        self.workers.push((worker, server.into()));

        let (worker, server) = self.workers.last().unwrap();
        Ok((worker, server.clone()))
    }

    fn get_existing_worker_and_server(&mut self) -> (&Worker, WebRtcServer) {
        let (worker, server) = &self.workers[self.next_worker];

        self.next_worker += 1;
        if self.next_worker >= self.workers.len() {
            self.next_worker = 0;
        }

        (worker, server.clone())
    }
}

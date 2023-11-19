use std::{
    fs::File,
    io::BufReader,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::{NonZeroU32, NonZeroU8},
    str::FromStr,
};

use lazy_static::lazy_static;
use log::{error, info, warn};
use mediasoup::{
    data_structures::Protocol,
    prelude::ListenIp,
    router::RouterOptions,
    rtp_parameters::{
        MimeTypeAudio, MimeTypeVideo, RtcpFeedback, RtpCodecCapability,
        RtpCodecParametersParameters,
    },
    webrtc_server::{
        WebRtcServer, WebRtcServerListenInfo, WebRtcServerListenInfos, WebRtcServerOptions,
    },
    webrtc_transport::WebRtcTransportOptions,
    worker::{WorkerLogLevel, WorkerSettings},
};
use rustls::ServerConfig;

// get and parse an environment variable
// use default value if not set
fn var<T>(name: &str, default: &str) -> T
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    let given = std::env::var(name).unwrap_or(default.to_owned());
    match given.parse() {
        Ok(parsed) => parsed,
        Err(e) => {
            error!(
                "Invalid config option `{}={}`: {:?} ({}'s default is usually {})",
                name, given, e, name, default
            );
            std::process::exit(1);
        }
    }
}

fn parse_range(range: &str) -> (u16, u16) {
    let mut split = range.split('-');

    let min = split.next().unwrap().parse().unwrap();
    let max = match split.next().and_then(|s| s.parse().ok()) {
        Some(max) => max,
        None => min,
    };

    (min, max)
}

lazy_static! {
    pub static ref NUM_WEB_WORKERS: usize = var("NUM_WEB_WORKERS", "4");

    static ref RTC_PORT_RANGE: (u16, u16) = {
        let ports: String = var("WRTC_PORTS", "10000");
        parse_range(&ports)
    };

    static ref ANNOUNCE_IP: IpAddr = IpAddr::V4(var("WRTC_ANNOUNCE_IP", "127.0.0.1"));

    static ref ENABLE_UDP: bool = var("WRTC_ENABLE_UDP", "true");
    static ref ENABLE_TCP: bool = var("WRTC_ENABLE_TCP", "true");
    static ref PREFER_UDP: bool = var("WRTC_PREFER_UDP", "true");
    static ref PREFER_TCP: bool = var("WRTC_PREFER_TCP", "false");

    static ref INITIAL_AVAILABLE_OUTGOING_BITRATE: u32 = var("WRTC_INITIAL_AVAILABLE_OUTGOING_BITRATE", "600000");

    static ref DB_HOST: String = var("DB_HOST", "127.0.0.1");
    static ref DB_PORT: u16 = var("DB_PORT", "5432");
    static ref DB_USER: String = var("DB_USER", "zling-backend");
    static ref DB_PASSWORD: String = var("DB_PASSWORD", "dev");
    static ref DB_NAME: String = var("DB_NAME", "zling-backend");
    static ref DB_POOL_MAX_CONNS: u32 = var("DB_POOL_MAX_CONNS", "5");
    pub static ref DB_RUN_MIGRATIONS: bool = var("DB_RUN_MIGRATIONS", "true");

    pub static ref BIND_ADDR: SocketAddr = var("BIND_ADDR", "127.0.0.1:8080");

    pub static ref SSL_ENABLE: bool = var("SSL_ENABLE", "false");
    pub static ref SSL_ONLY: bool = var("SSL_ONLY", "false");
    pub static ref SSL_BIND_ADDR: SocketAddr = var("SSL_BIND_ADDR", "127.0.0.1:8443");
    static ref SSL_CERT_PATH: String = var("SSL_CERT_PATH", "cert.pem");
    static ref SSL_KEY_PATH: String = var("SSL_KEY_PATH", "key.pem");

    pub static ref HANDLE_CORS: bool = var("HANDLE_CORS", "true");

    pub static ref MEDIA_PATH: String = {
        let path: String = var("MEDIA_PATH", "/var/tmp/zling-media");

        // create directory
        std::fs::create_dir_all(path.clone()).expect("failed to create directory specified by MEDIA_PATH");

        let is_read_only = std::fs::metadata(path.clone()).unwrap().permissions().readonly();
        if is_read_only {
            warn!("\n\nMEDIA_PATH directory at `{}` is not writable!\nUploads will probably fail!\n\n", path);
        }

        path
    };

    pub static ref TOKEN_SIGNING_KEY: [u8; 32] = {
        let tsk: String = var("TOKEN_SIGNING_KEY", "");

        if tsk.is_empty() {
            info!("Generating new token signing key... (provide one with TOKEN_SIGNING_KEY)");
            let generated = crate::crypto::generate_token_sig_key();
            info!("Token signing key: {}", hex::encode(generated));
            generated
        } else {
            let key = hex::decode(tsk).unwrap();
            if key.len() != 32 {
                error!("Invalid token signing key length, must be 32 bytes");
                std::process::exit(1);
            }
            key.try_into().unwrap()
        }
    };
}

pub fn media_codecs() -> Vec<RtpCodecCapability> {
    vec![
        RtpCodecCapability::Audio {
            mime_type: MimeTypeAudio::Opus,
            preferred_payload_type: None,
            clock_rate: NonZeroU32::new(48000).unwrap(),
            channels: NonZeroU8::new(2).unwrap(),
            parameters: RtpCodecParametersParameters::from([("useinbandfec", 1_u32.into())]),
            rtcp_feedback: vec![RtcpFeedback::TransportCc],
        },
        RtpCodecCapability::Video {
            mime_type: MimeTypeVideo::Vp9,
            preferred_payload_type: None,
            clock_rate: NonZeroU32::new(90000).unwrap(),
            parameters: RtpCodecParametersParameters::default(),
            rtcp_feedback: vec![
                RtcpFeedback::Nack,
                RtcpFeedback::NackPli,
                RtcpFeedback::CcmFir,
                RtcpFeedback::GoogRemb,
                RtcpFeedback::TransportCc,
            ],
        },
    ]
}

pub fn worker_settings() -> WorkerSettings {
    let mut worker_settings = WorkerSettings::default();
    worker_settings.log_level = WorkerLogLevel::Warn;
    worker_settings.rtc_ports_range = (RTC_PORT_RANGE.0)..=(RTC_PORT_RANGE.1);
    worker_settings
}

pub fn router_options() -> RouterOptions {
    RouterOptions::new(media_codecs())
}

pub fn webrtc_transport_options(webrtc_server: WebRtcServer) -> WebRtcTransportOptions {
    let mut opts = WebRtcTransportOptions::new_with_server(webrtc_server);

    opts.enable_udp = *ENABLE_UDP;
    opts.enable_tcp = *ENABLE_TCP;
    opts.prefer_udp = *PREFER_UDP;
    opts.prefer_tcp = *PREFER_TCP;
    opts.initial_available_outgoing_bitrate = *INITIAL_AVAILABLE_OUTGOING_BITRATE;

    opts
}

pub fn webrtc_server_options(port: u16) -> WebRtcServerOptions {
    let listen_ip = ListenIp {
        ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        announced_ip: Some(*ANNOUNCE_IP),
    };

    let mut listen_infos = Vec::with_capacity(2);

    if *ENABLE_UDP {
        listen_infos.push(WebRtcServerListenInfo {
            listen_ip,
            port: Some(port),
            protocol: Protocol::Udp,
        });
    }

    if *ENABLE_TCP {
        listen_infos.push(WebRtcServerListenInfo {
            listen_ip,
            port: Some(port),
            protocol: Protocol::Tcp,
        });
    }

    let mut wrtc_listen_infos = WebRtcServerListenInfos::new(listen_infos.pop().unwrap());
    if let Some(listen_info) = listen_infos.pop() {
        // god I hate mediasoup's rust API sometimes
        wrtc_listen_infos = wrtc_listen_infos.insert(listen_info);
    }

    WebRtcServerOptions::new(wrtc_listen_infos)
}

pub fn voice_ports() -> Vec<u16> {
    (RTC_PORT_RANGE.0..=RTC_PORT_RANGE.1).rev().collect()
}

pub fn db_conn_string() -> String {
    format!(
        "postgres://{}:{}@{}:{}/{}",
        *DB_USER, *DB_PASSWORD, *DB_HOST, *DB_PORT, *DB_NAME
    )
}

pub fn bind_addr() -> (IpAddr, u16) {
    (BIND_ADDR.ip(), BIND_ADDR.port())
}

pub fn ssl_bind_addr() -> (IpAddr, u16) {
    (SSL_BIND_ADDR.ip(), SSL_BIND_ADDR.port())
}

/// Load the SSL certificate and key files into a rustls config object
///
/// Taken from https://github.com/actix/examples/blob/master/https-tls/rustls/src/main.rs
pub fn ssl_config() -> rustls::ServerConfig {
    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = &mut BufReader::new(File::open(&*SSL_CERT_PATH).unwrap());
    let key_file = &mut BufReader::new(File::open(&*SSL_KEY_PATH).unwrap());

    // convert files to key/cert objects
    let cert_chain = rustls_pemfile::certs(cert_file)
        .unwrap()
        .into_iter()
        .map(rustls::Certificate)
        .collect();

    let mut keys: Vec<rustls::PrivateKey> = rustls_pemfile::pkcs8_private_keys(key_file)
        .unwrap()
        .into_iter()
        .map(rustls::PrivateKey)
        .collect();

    // exit if no keys could be parsed
    if keys.is_empty() {
        error!("Could not locate SSL private key at {}", *SSL_KEY_PATH);
        std::process::exit(1);
    }

    config.with_single_cert(cert_chain, keys.remove(0)).unwrap()
}

pub fn initialize_all() {
    lazy_static::initialize(&RTC_PORT_RANGE);

    if RTC_PORT_RANGE.0 > RTC_PORT_RANGE.1 {
        error!("Minimum RTC port cannot be greater than maximum RTC port");
        std::process::exit(1);
    }

    lazy_static::initialize(&ANNOUNCE_IP);
    lazy_static::initialize(&INITIAL_AVAILABLE_OUTGOING_BITRATE);

    lazy_static::initialize(&ENABLE_UDP);
    lazy_static::initialize(&ENABLE_TCP);
    lazy_static::initialize(&PREFER_UDP);
    lazy_static::initialize(&PREFER_TCP);

    if *PREFER_TCP == *PREFER_UDP {
        error!("PREFER_TCP and PREFER_UDP cannot both be true or both be false");
        std::process::exit(1);
    }

    if !*ENABLE_TCP && *PREFER_TCP {
        error!("PREFER_TCP cannot be true if ENABLE_TCP is false");
        std::process::exit(1);
    }

    if !*ENABLE_UDP && *PREFER_UDP {
        error!("PREFER_UDP cannot be true if ENABLE_UDP is false");
        std::process::exit(1);
    }

    if !*SSL_ENABLE && *SSL_ONLY {
        error!("SSL_ONLY cannot be true if SSL_ENABLE is false");
        std::process::exit(1);
    }

    lazy_static::initialize(&BIND_ADDR);

    lazy_static::initialize(&SSL_BIND_ADDR);
    lazy_static::initialize(&SSL_CERT_PATH);
    lazy_static::initialize(&SSL_KEY_PATH);

    lazy_static::initialize(&DB_HOST);
    lazy_static::initialize(&DB_PORT);
    lazy_static::initialize(&DB_USER);
    lazy_static::initialize(&DB_PASSWORD);
    lazy_static::initialize(&DB_NAME);
    lazy_static::initialize(&DB_POOL_MAX_CONNS);
    lazy_static::initialize(&TOKEN_SIGNING_KEY);

    lazy_static::initialize(&MEDIA_PATH);
}

pub fn print_all() {
    let port_range = RTC_PORT_RANGE.1 - RTC_PORT_RANGE.0 + 1;

    if port_range == 1 {
        info!(
            "config: WebRTC Port: {} (will spawn 1 WebRTC server)",
            RTC_PORT_RANGE.0
        );
    } else {
        info!(
            "config: WebRTC Ports: {}-{} (will spawn {} WebRTC servers)",
            RTC_PORT_RANGE.0, RTC_PORT_RANGE.1, port_range
        );

        let num_cpus = std::thread::available_parallelism()
            .unwrap_or(1usize.try_into().unwrap())
            .get();

        if port_range > 4 && num_cpus < port_range as usize {
            warn!(
                "Each voice port will spawn its own WebRTC server! You have captured {} ports with WRTC_PORTS, but you only have {} CPU threads. You may want to decrease RTC_PORT_MAX to {} or less.", 
                port_range,
                num_cpus,
                (RTC_PORT_RANGE.0 as usize) + num_cpus - 1
            );
        }
    }

    info!("config: WebRTC Announce IP: {}", *ANNOUNCE_IP);

    if ANNOUNCE_IP.is_loopback() {
        warn!("WRTC_ANNOUNCE_IP is set to a loopback address, voice clients will probably not be able to connect! Set it to your server's public IP!")
    }

    info!(
        "config: WebRTC Initial Available Outgoing Bitrate: {}bps",
        *INITIAL_AVAILABLE_OUTGOING_BITRATE
    );
    info!(
        "config: WebRTC UDP Enabled: {}",
        if *ENABLE_UDP { "yes" } else { "no" }
    );
    info!(
        "config: WebRTC TCP Enabled: {}",
        if *ENABLE_TCP { "yes" } else { "no" }
    );
    info!(
        "config: WebRTC Preferred Protocol: {}",
        if *PREFER_UDP { "UDP" } else { "TCP" }
    );
    info!(
        "config: Database: {} at {}:{} ({} max connections)",
        *DB_NAME, *DB_HOST, *DB_PORT, *DB_POOL_MAX_CONNS
    );

    info!("config: Uploaded media stored in: {}", *MEDIA_PATH);
}

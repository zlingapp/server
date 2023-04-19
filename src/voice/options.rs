use std::{
    net::{IpAddr, Ipv4Addr},
    num::{NonZeroU32, NonZeroU8},
    str::FromStr,
};

use lazy_static::lazy_static;
use log::info;
use mediasoup::{
    prelude::ListenIp,
    router::RouterOptions,
    rtp_parameters::{
        MimeTypeAudio, MimeTypeVideo, RtcpFeedback, RtpCodecCapability,
        RtpCodecParametersParameters,
    },
    webrtc_transport::{TransportListenIps, WebRtcTransportOptions},
    worker::{WorkerLogLevel, WorkerSettings},
};

// get and parse an environment variable
// use default value if not set
fn var<T>(name: &str, default: &str) -> T
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Debug,
{
    std::env::var(name)
        .unwrap_or(default.to_owned())
        .parse()
        .unwrap()
}

lazy_static! {
    /// This is an example for using doc comment attributes
    static ref RTC_PORT_MIN: u16 = var("RTC_PORT_MIN", "10000");
    static ref RTC_PORT_MAX: u16 = var("RTC_PORT_MAX", "11000");
    
    static ref ANNOUNCE_IP: IpAddr = IpAddr::V4(var("ANNOUNCE_IP", "127.0.0.1"));

    static ref ENABLE_UDP: bool = var("ENABLE_UDP", "true");
    static ref ENABLE_TCP: bool = var("ENABLE_TCP", "true");
    static ref PREFER_UDP: bool = var("PREFER_UDP", "true");
    static ref PREFER_TCP: bool = var("PREFER_TCP", "false");

    static ref INITIAL_AVAILABLE_OUTGOING_BITRATE: u32 = var("INITIAL_AVAILABLE_OUTGOING_BITRATE", "600000");
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
            mime_type: MimeTypeVideo::Vp8,
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
    worker_settings.rtc_ports_range = (*RTC_PORT_MIN)..=(*RTC_PORT_MAX);
    worker_settings
}

pub fn router_options() -> RouterOptions {
    RouterOptions::new(media_codecs())
}

pub fn webrtc_transport_options() -> WebRtcTransportOptions {
    let mut opts = WebRtcTransportOptions::new(TransportListenIps::new(ListenIp {
        ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        announced_ip: Some(*ANNOUNCE_IP),
    }));

    opts.enable_udp = *ENABLE_UDP;
    opts.enable_tcp = *ENABLE_TCP;
    opts.prefer_udp = *PREFER_UDP;
    opts.prefer_tcp = *PREFER_TCP;
    opts.initial_available_outgoing_bitrate = *INITIAL_AVAILABLE_OUTGOING_BITRATE;

    opts
}

pub fn initialize_all() {
    lazy_static::initialize(&RTC_PORT_MIN);
    lazy_static::initialize(&RTC_PORT_MAX);
    lazy_static::initialize(&ANNOUNCE_IP);
    lazy_static::initialize(&INITIAL_AVAILABLE_OUTGOING_BITRATE);
    
    lazy_static::initialize(&ENABLE_UDP);
    lazy_static::initialize(&ENABLE_TCP);
    lazy_static::initialize(&PREFER_UDP);
    lazy_static::initialize(&PREFER_TCP);

    if *PREFER_TCP == *PREFER_UDP {
        panic!("PREFER_TCP and PREFER_UDP cannot both be true or both be false");
    }

    if !*ENABLE_TCP && *PREFER_TCP {
        panic!("PREFER_TCP cannot be true if ENABLE_TCP is false");
    }

    if !*ENABLE_UDP && *PREFER_UDP {
        panic!("PREFER_UDP cannot be true if ENABLE_UDP is false");
    }
}

pub fn print_all() {
    info!("config: RTC Ports: {}-{}", *RTC_PORT_MIN, *RTC_PORT_MAX);
    info!("config: Announce IP: {}", *ANNOUNCE_IP);
    info!("config: Initial Available Outgoing Bitrate: {}bps", *INITIAL_AVAILABLE_OUTGOING_BITRATE);
    info!("config: UDP Enabled: {}", *ENABLE_UDP);
    info!("config: TCP Enabled: {}", *ENABLE_TCP);
    info!("config: Preferred: {}", if *PREFER_UDP { "UDP" } else { "TCP" });
}
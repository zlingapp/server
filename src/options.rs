use std::{
    net::{IpAddr, Ipv4Addr},
    num::{NonZeroU32, NonZeroU8},
};

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
    worker_settings.rtc_ports_range=10000..=10010;
    worker_settings
}

pub fn router_options() -> RouterOptions {
    RouterOptions::new(media_codecs())
}

pub fn webrtc_transport_options() -> WebRtcTransportOptions {
    // get environment variable for announce ip
    let announce_ip = std::env::var("PUBLIC_IP")
        .unwrap_or_else(|_| "127.0.0.1".to_owned())
        .parse()
        .unwrap();

    // info!("RTC announce IP: {}", announce_ip);

    let mut opts = WebRtcTransportOptions::new(TransportListenIps::new(ListenIp {
        ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        announced_ip: Some(IpAddr::V4(announce_ip)),
    }));

    opts.enable_udp = true;
    opts.enable_tcp = true;
    opts.prefer_udp = true;

    opts
}

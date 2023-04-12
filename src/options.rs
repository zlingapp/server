use std::{num::{NonZeroU32, NonZeroU8}, net::{IpAddr, Ipv4Addr}};

use mediasoup::{rtp_parameters::{
    MimeTypeAudio, MimeTypeVideo, RtcpFeedback, RtpCodecCapability, RtpCodecParametersParameters,
}, router::RouterOptions, worker::WorkerSettings, webrtc_transport::{WebRtcTransportOptions, TransportListenIps}, prelude::ListenIp};

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
    WorkerSettings::default()
}

pub fn router_options() -> RouterOptions {
    RouterOptions::new(media_codecs())
}

pub fn webrtc_transport_options() -> WebRtcTransportOptions {
    let mut opts = WebRtcTransportOptions::new(
        TransportListenIps::new(
            ListenIp {
                ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                announced_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            }
        )
    );

    opts.enable_udp = true;
    opts.enable_tcp = true;
    opts.prefer_udp = true;

    opts
}
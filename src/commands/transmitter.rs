use std::{env, thread};
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender};
use std::time::Instant;

use rodio::buffer::SamplesBuffer;
use rodio::dynamic_mixer::mixer;
use rodio::source::UniformSourceIterator;
use serenity::async_trait;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};

use dmr_bridge_discord::packet::USRP;

pub struct Transmitter {
    tx: Sender<Vec<i16>>,
}

pub struct TransmitterWrapper {
    transmitter: Arc<Transmitter>,
}

impl TransmitterWrapper {
    pub fn new(transmitter: Arc<Transmitter>) -> Self {
        Self { transmitter }
    }
}

#[async_trait]
impl VoiceEventHandler for TransmitterWrapper {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        self.transmitter.act(ctx).await
    }
}

impl Transmitter {
    pub fn new() -> Self {
        // You can manage state here, such as a buffer of audio packet bytes, so
        // you can later store them in intervals.
        let dmr_target_rx_addr =
            env::var("TARGET_RX_ADDR").expect("Expected a target rx address in the environment");

        let socket =
            UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind udp socket for transmission");

        socket
            .connect(dmr_target_rx_addr)
            .expect("Couldn't connect to DMR audio receiver");
        socket.set_nonblocking(true).expect("TODO: panic message");

        let (mixer_controller, mut mixer) = mixer(1, 8000);
        let (tx, rx) = channel::<Vec<i16>>();

        thread::spawn(move || loop {
            if let Ok(audio_packet) = rx.recv() {
                let source = SamplesBuffer::new(2, 48000, audio_packet);
                let uniform_source = UniformSourceIterator::new(source, 1, 8000);
                let mut uniform_source_data: Vec<i16> = uniform_source.collect();
                uniform_source_data.resize(160, 0);
                let source = SamplesBuffer::new(1, 8000, uniform_source_data);
                mixer_controller.add(source);
            }
        });

        thread::spawn(move || {
            let mut sequence = 0;
            let mut last_time_streaming_audio: Option<Instant> = None;

            loop {
                let mut audio: Vec<i16> = mixer.by_ref().take(160).collect();
                if !audio.is_empty() {
                    audio.resize(160, 0);
                    let mut audio_packet = [0i16; 160];
                    audio_packet.clone_from_slice(&audio);
                    let usrp_packet = USRP {
                        sequence_counter: sequence,
                        push_to_talk: true,
                        audio: audio_packet,
                        ..Default::default()
                    };
                    sequence += 1;
                    socket
                        .send(&usrp_packet.to_buffer())
                        .expect("Failed to send USRP voice audio packet");
                    last_time_streaming_audio = Some(Instant::now());
                }
                if last_time_streaming_audio.is_some()
                    && Instant::now()
                        .duration_since(last_time_streaming_audio.unwrap())
                        .as_millis()
                        > 200
                {
                    last_time_streaming_audio = None;
                    let usrp_packet = USRP {
                        sequence_counter: sequence,
                        ..Default::default()
                    };
                    sequence += 1;
                    socket
                        .send(&usrp_packet.to_buffer())
                        .expect("Failed to send USRP voice end packet");
                }
            }
        });

        println!("Transmitter started");

        Self { tx }
    }
}

#[async_trait]
impl VoiceEventHandler for Transmitter {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        use EventContext as Ctx;
        match ctx {
            Ctx::VoicePacket(data) => {
                if let Some(audio) = data.audio {
                    self.tx.send(audio.clone()).unwrap();
                }
            }
            _ => {
                unimplemented!()
            }
        }

        None
    }
}

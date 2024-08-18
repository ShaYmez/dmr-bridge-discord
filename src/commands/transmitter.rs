use std::{env, thread};
use std::fs::File;
use std::io::Write;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::time::Duration;

use byteorder::{ByteOrder, LittleEndian};
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

        let (tx, rx) = channel::<Vec<i16>>();

        let mut file = File::create("./audio.pcm").unwrap();
        let mut sequence: u32 = 0;
        let mut sent_audio = false;
        let mut buffer: Vec<i16> = Vec::new();

        thread::spawn(move || loop {
            let mut pulled_audio = Vec::with_capacity(160);
            let mut communication_end = false;
            while pulled_audio.len() < 160 {
                pulled_audio.extend(buffer.iter().by_ref().take(160 - pulled_audio.len()));
                if pulled_audio.len() != 160 {
                    match rx.recv_timeout(Duration::from_millis(200)) {
                        Ok(audio_packet) => {
                            buffer.extend(audio_packet);
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            if sent_audio {
                                communication_end = true;
                                println!("Communication ended");
                                break;
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            if !pulled_audio.is_empty() {
                let mut audio_packet = [0i16; 160];
                if pulled_audio.len() != 160 {
                    pulled_audio.resize(160, 0);
                }
                audio_packet.clone_from_slice(&pulled_audio);
                let usrp_packet = USRP {
                    sequence_counter: sequence,
                    push_to_talk: true,
                    audio: audio_packet,
                    ..Default::default()
                };
                /*socket
                .send(&usrp_packet.to_buffer())
                .expect("Failed to send USRP voice end packet");*/
                sent_audio = true;
                sequence += 1;
                let mut audio_packet = [0u8; 320];
                LittleEndian::write_i16_into(&usrp_packet.audio, &mut audio_packet);
                file.write_all(&audio_packet).expect("TODO: panic message");
            }

            if communication_end {
                let usrp_packet = USRP {
                    sequence_counter: sequence,
                    ..Default::default()
                };
                sequence += 1;
                /*socket
                .send(&usrp_packet.to_buffer())
                .expect("Failed to send USRP voice end packet");*/
                sent_audio = false;
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
            Ctx::VoiceTick(data) => {
                let (mixer_controller, mixer) = mixer(1, 8000);
                for (_, audio) in data.speaking.iter() {
                    let audio_packet = audio.decoded_voice.clone();
                    if let Some(audio_packet) = audio_packet {
                        let source = SamplesBuffer::new(2, 48000, audio_packet);
                        let uniform_source = UniformSourceIterator::new(source, 1, 8000);
                        let uniform_source_data: Vec<i16> = uniform_source.collect();
                        let source = SamplesBuffer::new(1, 8000, uniform_source_data);
                        mixer_controller.add(source);
                    }
                }
                let audio_packet: Vec<i16> = mixer.collect();
                match self.tx.send(audio_packet) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            _ => {
                unimplemented!()
            }
        }

        None
    }
}

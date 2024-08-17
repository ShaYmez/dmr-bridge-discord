use std::env;
use std::net::UdpSocket;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use byteorder::{ByteOrder, LittleEndian};
use futures::executor::block_on;
use rodio::buffer::SamplesBuffer;
use rodio::source::UniformSourceIterator;
use serenity::prelude::Mutex as SerenityMutex;
use songbird::{Call, input::Input};
use songbird::input::{Codec, Container, Reader};

use dmr_bridge_discord::packet::{PacketType, USRP};

pub struct Receiver {
    discord_channel: Arc<Mutex<Option<Arc<SerenityMutex<Call>>>>>,
}

impl Receiver {
    pub fn new() -> Self {
        // You can manage state here, such as a buffer of audio packet bytes so
        // you can later store them in intervals.
        let dmr_local_rx_addr =
            env::var("LOCAL_RX_ADDR").expect("Expected a local rx address in the environment");

        let socket =
            UdpSocket::bind(dmr_local_rx_addr).expect("Couldn't bind udp socket for reception");

        let discord_channel_mutex: Arc<Mutex<Option<Arc<SerenityMutex<Call>>>>> =
            Arc::new(Mutex::new(None));

        let discord_channel = Arc::clone(&discord_channel_mutex);

        let (tx, rx) = channel::<USRP>();

        thread::spawn(move || loop {
            if let Ok(usrp_packet) = rx.recv() {
                let source = SamplesBuffer::new(1, 8000, usrp_packet.audio);
                let uniform_source = UniformSourceIterator::new(source, 1, 48000);
                let mut uniform_source_data: Vec<i16> = uniform_source.collect();
                uniform_source_data.resize(960, 0);
                let mut audio_data: [u8; 1920] = [0; 1920];
                LittleEndian::write_i16_into(&uniform_source_data, &mut audio_data);
                let audio = Input::new(
                    false,
                    Reader::from(Vec::from(audio_data)),
                    Codec::Pcm,
                    Container::Raw,
                    None,
                );
                if let Ok(channel) = discord_channel.lock() {
                    if let Some(device) = channel.deref() {
                        let mut call = block_on(device.lock());
                        let handle = call.play_source(audio);
                        let duration = handle.metadata().duration.unwrap_or(Duration::from_millis(20));
                        thread::sleep(duration);
                    }
                }
            }
        });

        thread::spawn(move || {
            let mut buffer = [0u8; 352];
            loop {
                match socket.recv(&mut buffer) {
                    Ok(32..) => {
                        if let Some(usrp_packet) = USRP::from_buffer(buffer) {
                            if usrp_packet.packet_type == PacketType::Voice {
                                tx.send(usrp_packet).unwrap();
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => return,
                }
            }
        });

        println!("Receiver started");

        Self {
            discord_channel: discord_channel_mutex,
        }
    }

    pub fn set(&mut self, device: Arc<SerenityMutex<Call>>) {
        let device = Arc::clone(&device);
        let mut discord_channel = self.discord_channel.lock().unwrap();
        *discord_channel = Some(device);

        println!("Receiver has been associated with a discord channel");
    }

    pub fn unset(&mut self) {
        let mut discord_channel = self.discord_channel.lock().unwrap();
        *discord_channel = None;

        println!("Receiver has been disassociated from a discord channel");
    }
}

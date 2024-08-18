use std::env;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use byteorder::{ByteOrder, LittleEndian};
use futures::executor::block_on;
use rodio::buffer::SamplesBuffer;
use rodio::source::UniformSourceIterator;
use serenity::prelude::Mutex as SerenityMutex;
use songbird::Call;
use songbird::input::Input;

use dmr_bridge_discord::packet::{PacketType, USRP};

pub struct Receiver;

fn i16_samples_to_f32_samples(samples: Vec<i16>) -> Vec<f32> {
    samples.iter().map(|&sample| (sample as f32) / (i16::MIN as f32).abs()).collect()
}

impl Receiver {
    pub fn new(call: Arc<SerenityMutex<Call>>) -> Self {
        let (tx, rx) = channel::<USRP>();

        thread::spawn(move || loop {
            if let Ok(usrp_packet) = rx.recv() {
                let source = SamplesBuffer::new(1, 8000, usrp_packet.audio);
                let uniform_source = UniformSourceIterator::new(source, 1, 48000);
                let mut uniform_source_data: Vec<i16> = uniform_source.collect();
                uniform_source_data.resize(960, 0);
                let audio =  i16_samples_to_f32_samples(uniform_source_data);
                let mut audio_data: [u8; 3840] = [0; 3840];
                LittleEndian::write_f32_into(&audio, &mut audio_data);

                let input = Input::from(audio_data);
                let mut call = block_on(call.lock());
                call.play_input(input);
                thread::sleep(Duration::from_millis(20));

            }
        });

        let dmr_local_rx_addr =
            env::var("LOCAL_RX_ADDR").expect("Expected a local rx address in the environment");
        let socket =
            UdpSocket::bind(dmr_local_rx_addr).expect("Couldn't bind udp socket for reception");
        let mut buffer = [0u8; 352];
        thread::spawn(move || loop {
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
        });

        println!("Receiver started");

        Self {}
    }
}

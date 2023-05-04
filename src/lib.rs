pub mod packet;

#[derive(PartialEq, Debug)]
pub enum USRPVoicePacketType {
    Start,
    Audio,
    End,
}

use byteorder::{BigEndian, ByteOrder, LittleEndian};

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum PacketType {
    Voice = 0,
    DualToneMultiFrequency = 1,
    Text = 2,
    Unknown = u32::MAX,
}

impl From<u32> for PacketType {
    fn from(value: u32) -> Self {
        match value {
            0 => PacketType::Voice,
            1 => PacketType::DualToneMultiFrequency,
            2 => PacketType::Text,
            _ => PacketType::Unknown,
        }
    }
}

impl From<PacketType> for u32 {
    fn from(value: PacketType) -> Self {
        match value {
            PacketType::Voice => 0,
            PacketType::DualToneMultiFrequency => 1,
            PacketType::Text => 2,
            PacketType::Unknown => u32::MAX,
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
pub struct USRP {
    // pub magic_word: [u8; 4],
    pub sequence_counter: u32,
    pub stream_id: u32,
    pub push_to_talk: bool,
    pub talk_group: u32,
    pub packet_type: PacketType,
    pub multiplex_id: u32,
    pub reserved: u32,
    pub audio: [i16; 160],
}

impl Default for USRP {
    fn default() -> Self {
        USRP {
            sequence_counter: 0,
            stream_id: 0,
            push_to_talk: false,
            talk_group: 0,
            packet_type: PacketType::Voice,
            multiplex_id: 0,
            reserved: 0,
            audio: [0i16; 160],
        }
    }
}

impl USRP {
    pub fn from_buffer(buffer: [u8; 352]) -> Option<Self> {
        if &buffer[0..4] != b"USRP" {
            println!("Invalid packet: {:?}", buffer);
            None
        } else {
            let mut audio = [0i16; 160];
            LittleEndian::read_i16_into(&buffer[32..352], &mut audio);
            Some(Self {
                sequence_counter: BigEndian::read_u32(&buffer[4..8]),
                stream_id: BigEndian::read_u32(&buffer[8..12]),
                push_to_talk: BigEndian::read_u32(&buffer[12..16]) != 0,
                talk_group: BigEndian::read_u32(&buffer[16..20]),
                packet_type: LittleEndian::read_u32(&buffer[20..24]).into(),
                multiplex_id: BigEndian::read_u32(&buffer[24..28]),
                reserved: BigEndian::read_u32(&buffer[28..32]),
                audio,
            })
        }
    }

    pub fn to_buffer(&self) -> [u8; 352] {
        let mut buffer = [0u8; 352];
        buffer[..4].copy_from_slice(b"USRP");
        BigEndian::write_u32(&mut buffer[4..8], self.sequence_counter);
        BigEndian::write_u32(&mut buffer[8..12], self.stream_id);
        BigEndian::write_u32(&mut buffer[12..16], self.push_to_talk.into());
        BigEndian::write_u32(&mut buffer[16..20], self.talk_group);
        LittleEndian::write_u32(&mut buffer[20..24], self.packet_type.into());
        BigEndian::write_u32(&mut buffer[24..28], self.multiplex_id);
        BigEndian::write_u32(&mut buffer[28..32], self.reserved);
        LittleEndian::write_i16_into(&self.audio, &mut buffer[32..352]);
        buffer
    }
}

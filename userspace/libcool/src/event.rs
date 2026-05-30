use super::{io, Error, Result};

pub const INPUT_FD: u64 = 3;
pub const EVENT_PACKET_SIZE: usize = 8;
pub const EVENT_KIND_KEY_CHAR: u8 = 1;
pub const EVENT_KIND_MOUSE_DOWN: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    KeyChar { bytes: [u8; 4], len: usize },
    MouseDown { x: u16, y: u16 },
}

impl Event {
    pub fn parse(packet: &[u8; EVENT_PACKET_SIZE]) -> Result<Self> {
        match packet[0] {
            EVENT_KIND_KEY_CHAR => {
                let len = packet[1] as usize;
                if len == 0 || len > 4 {
                    return Err(Error::Invalid);
                }
                let mut bytes = [0u8; 4];
                bytes[..len].copy_from_slice(&packet[2..2 + len]);
                Ok(Event::KeyChar { bytes, len })
            }
            EVENT_KIND_MOUSE_DOWN => Ok(Event::MouseDown {
                x: u16::from_le_bytes([packet[2], packet[3]]),
                y: u16::from_le_bytes([packet[4], packet[5]]),
            }),
            _ => Err(Error::Invalid),
        }
    }
}

pub fn read_event(fd: u64) -> Result<Option<Event>> {
    let mut packet = [0u8; EVENT_PACKET_SIZE];
    let n = io::read(fd, &mut packet)?;
    if n == 0 {
        return Ok(None);
    }
    if n != EVENT_PACKET_SIZE {
        return Err(Error::Invalid);
    }
    Event::parse(&packet).map(Some)
}

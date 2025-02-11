use bytes::BytesMut;
use tokio::io;

use tokio_util::codec::{Decoder, Encoder};

use coap_lite::Packet;

pub struct Codec {}

impl Codec {
    pub fn new() -> Codec {
        Codec {}
    }
}

impl Default for Codec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for Codec {
    type Item = Packet;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Packet>, io::Error> {
        if buf.is_empty() {
            return Ok(None);
        }
        let result = (|| {
            Ok(Some(Packet::from_bytes(buf).map_err(|cause| {
                io::Error::new(io::ErrorKind::InvalidData, cause.to_string())
            })?))
        })();
        buf.clear();
        result
    }
}

impl Encoder<Packet> for Codec {
    type Error = io::Error;

    fn encode(&mut self, my_packet: Packet, buf: &mut BytesMut) -> Result<(), io::Error> {
        buf.extend_from_slice(&my_packet.to_bytes()
        .map_err(|cause| io::Error::new(io::ErrorKind::InvalidData, cause.to_string()))?[..]);
        Ok(())
    }
}

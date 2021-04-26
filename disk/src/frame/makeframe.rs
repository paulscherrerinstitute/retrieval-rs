use crate::cache::BinnedBytesForHttpStreamFrame;
use crate::raw::conn::RawConnOut;
use crate::raw::EventQueryJsonStringFrame;
use bytes::{BufMut, BytesMut};
use err::Error;
use serde::Serialize;

pub const INMEM_FRAME_HEAD: usize = 16;
pub const INMEM_FRAME_MAGIC: u32 = 0xc6c3b73d;

pub trait FrameType {
    const FRAME_TYPE_ID: u32;
}

impl FrameType for BinnedBytesForHttpStreamFrame {
    const FRAME_TYPE_ID: u32 = 0x02;
}

impl FrameType for EventQueryJsonStringFrame {
    const FRAME_TYPE_ID: u32 = 0x03;
}

impl FrameType for RawConnOut {
    const FRAME_TYPE_ID: u32 = 0x04;
}

pub fn make_frame<FT>(item: &FT) -> Result<BytesMut, Error>
where
    FT: FrameType + Serialize,
{
    match bincode::serialize(item) {
        Ok(enc) => {
            if enc.len() > u32::MAX as usize {
                return Err(Error::with_msg(format!("too long payload {}", enc.len())));
            }
            let encid = 0x12121212;
            let mut buf = BytesMut::with_capacity(enc.len() + INMEM_FRAME_HEAD);
            buf.put_u32_le(INMEM_FRAME_MAGIC);
            buf.put_u32_le(encid);
            buf.put_u32_le(FT::FRAME_TYPE_ID);
            buf.put_u32_le(enc.len() as u32);
            buf.put(enc.as_ref());
            Ok(buf)
        }
        Err(e) => Err(e)?,
    }
}

pub fn make_term_frame() -> BytesMut {
    let encid = 0x12121313;
    let mut buf = BytesMut::with_capacity(INMEM_FRAME_HEAD);
    buf.put_u32_le(INMEM_FRAME_MAGIC);
    buf.put_u32_le(encid);
    buf.put_u32_le(0x01);
    buf.put_u32_le(0);
    buf
}
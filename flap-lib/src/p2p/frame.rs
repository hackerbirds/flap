use bytes::{Buf, BufMut, Bytes};

use crate::{
    crypto::{blake3::FileHash, encryption_stream::MAX_NOISE_MESSAGE_LENGTH},
    error::{Error, Result},
    fs::metadata::FlapFileMetadata,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    // msg = 0x01
    FileData(Bytes),
    // msg = 0x02
    PleaseSendEntireFile,
    // msg = 0x03
    IWillSendEntireFile(FlapFileMetadata),
    // msg = 0x04
    TransferComplete(FileHash),
}

pub(crate) const MAX_FRAME_OPTIONAL_DATA_SIZE: usize = MAX_NOISE_MESSAGE_LENGTH - size_of::<u8>();

/// [(u8)(data)]
/// (u8) is [`Message`]
/// (data) is optional data according to u8
/// Note: Size of `SerializedFrame` is always equal or less than 65535 (fits in u16)
pub type SerializedFrame = Vec<u8>;

impl Frame {
    pub fn to_bytes(&self) -> SerializedFrame {
        let mut vec: Vec<u8> = Vec::with_capacity(MAX_NOISE_MESSAGE_LENGTH);
        match self {
            Frame::FileData(bytes) => {
                debug_assert!(bytes.len() <= MAX_FRAME_OPTIONAL_DATA_SIZE);
                vec.put_u8(0x01);
                vec.put_slice(bytes.as_ref());
            }
            Frame::PleaseSendEntireFile => {
                vec.put_u8(0x02);
            }
            Frame::IWillSendEntireFile(flap_file_metadata) => {
                let bytes = flap_file_metadata.to_bytes();
                debug_assert!(bytes.len() <= MAX_FRAME_OPTIONAL_DATA_SIZE);
                vec.put_u8(0x03);
                vec.put_slice(bytes.as_ref());
            }
            Frame::TransferComplete(file_hash) => {
                vec.put_u8(0x04);
                vec.put_slice(file_hash);
            }
        }

        debug_assert!(vec.len() <= MAX_NOISE_MESSAGE_LENGTH);
        vec
    }

    pub async fn read_from_frame(mut frame: Bytes) -> Result<Self> {
        debug_assert!(frame.len() >= size_of::<u8>());
        debug_assert!(frame.len() <= MAX_NOISE_MESSAGE_LENGTH);

        let msg = frame.get_u8();

        match msg {
            0x01 => Ok(Self::FileData(frame)),
            0x02 => Ok(Self::PleaseSendEntireFile),
            0x03 => {
                let metadata = FlapFileMetadata::from_bytes(frame).await;
                Ok(Self::IWillSendEntireFile(metadata))
            }
            0x04 => Ok(Self::TransferComplete(
                frame
                    .as_ref()
                    .try_into()
                    .map_err(|_| Error::InvalidBlake3Hash)?,
            )),
            _ => panic!("Invalid message header"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    pub async fn basic_frame_roundtrip() {
        let frame1 = Frame::PleaseSendEntireFile;

        let frame2 = Frame::IWillSendEntireFile(FlapFileMetadata {
            is_file: true,
            dir_file_entries: None,
            file_size: 1,
            file_name: "a".to_string(),
        });

        let frame3 = Frame::PleaseSendEntireFile;

        /* roundtrip */
        let frame1_roundtrip = Frame::read_from_frame(frame1.to_bytes().into())
            .await
            .unwrap();
        let frame2_roundtrip = Frame::read_from_frame(frame2.to_bytes().into())
            .await
            .unwrap();
        let frame3_roundtrip = Frame::read_from_frame(frame3.to_bytes().into())
            .await
            .unwrap();

        assert_eq!(frame1_roundtrip, frame1);
        assert_eq!(frame2_roundtrip, frame2);
        assert_eq!(frame3_roundtrip, frame3);
    }
}

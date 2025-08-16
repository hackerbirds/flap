use std::io::ErrorKind;

use base64ct::Encoding;
use bytes::BytesMut;
use iroh::{
    PublicKey, SecretKey,
    endpoint::{RecvStream, SendStream, VarInt},
};
use snow::TransportState;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    crypto::{blake3::Blake3, transfer_id::TransferId, x25519},
    error::{Error, Result},
    fs::metadata::FlapFileMetadata,
    p2p::frame::Frame,
    ticket::Ticket,
};

static NOISE_PATTERN: &'static str = "Noise_KKhfs+psk2_25519+Kyber1024_ChaChaPoly_BLAKE2s";
pub(crate) const MAX_NOISE_MESSAGE_LENGTH: usize = u16::MAX as usize;

pub struct EncryptionStream {
    /// The stream to encrypt to.
    send_stream: SendStream,
    /// The stream to decrypt from.
    recv_stream: RecvStream,
    send_buffer: Vec<u8>,
    recv_buffer: Vec<u8>,
    file_hash: Blake3,
    noise: TransportState,
    transfer_id: TransferId,
}

impl EncryptionStream {
    pub async fn initiate(
        is_sender: bool,
        iroh_secret_key: &SecretKey,
        remote_public_key: &PublicKey,
        mut send_stream: SendStream,
        mut recv_stream: RecvStream,
        ticket: &Ticket,
    ) -> Result<Self> {
        let mut send_buffer = vec![0u8; MAX_NOISE_MESSAGE_LENGTH];
        let mut recv_buffer = vec![0u8; MAX_NOISE_MESSAGE_LENGTH];
        let file_hash = Blake3::default();

        let stream_id = VarInt::from(send_stream.id()).into_inner().to_be_bytes();
        let file_key = ticket.master_key().file_key();

        let local_x25519_private = x25519::iroh_secret_to_x25519_secret(iroh_secret_key);
        let remote_x25519_public = x25519::iroh_public_to_x25519_public(remote_public_key);

        let initiator = snow::Builder::new(NOISE_PATTERN.parse().unwrap())
            .local_private_key(local_x25519_private.as_bytes())
            .expect("has not been called previously")
            .remote_public_key(remote_x25519_public.as_bytes())
            .expect("has not been called previously")
            .prologue(&stream_id)
            .expect("has not been called previously")
            .psk(2, file_key.as_bytes())
            .expect("psk is 32 bytes long and has valid location");

        // Due to QUIC, sender needs to be initiator
        let handshake_state = if is_sender {
            // Receiver initiates the connection
            let mut handshake_state = initiator.build_initiator()?;

            let len = handshake_state.write_message(&[], &mut send_buffer)?;
            Self::send_msg(&mut send_stream, &mut send_buffer[0..len]).await?;

            // wait for handshake response
            let len = Self::recv_msg(&mut recv_stream, &mut recv_buffer).await?;
            handshake_state.read_message(&recv_buffer[0..len], &mut send_buffer)?;

            debug_assert!(handshake_state.is_handshake_finished());

            handshake_state
        } else {
            let mut handshake_state = initiator.build_responder()?;

            // wait for handshake
            let len = Self::recv_msg(&mut recv_stream, &mut recv_buffer).await?;
            handshake_state.read_message(&recv_buffer[0..len], &mut send_buffer)?;

            // send response
            let len = handshake_state.write_message(&[], &mut send_buffer)?;
            Self::send_msg(&mut send_stream, &mut send_buffer[0..len]).await?;

            debug_assert!(handshake_state.is_handshake_finished());

            handshake_state
        };

        let transfer_id = TransferId(
            handshake_state
                .get_handshake_hash()
                .try_into()
                .expect("hash len is 32"),
        );

        let noise = handshake_state.into_transport_mode()?;

        println!("Noise handshake successful");

        Ok(Self {
            send_stream,
            recv_stream,
            file_hash,
            send_buffer,
            recv_buffer,
            noise,
            transfer_id,
        })
    }

    #[inline]
    async fn send_msg(send_stream: &mut SendStream, send_buffer: &mut [u8]) -> Result<()> {
        send_stream.write_u16(send_buffer.len() as u16).await?;
        send_stream.write_all(&send_buffer).await?;
        send_stream.flush().await?;

        Ok(())
    }

    #[inline]
    async fn recv_msg(recv_stream: &mut RecvStream, recv_buffer: &mut Vec<u8>) -> Result<usize> {
        let msg_len = recv_stream.read_u16().await?;

        recv_stream
            .read_exact(&mut (recv_buffer[0..msg_len as usize]))
            .await?;

        Ok(msg_len as usize)
    }

    pub fn set_file_hasher(&mut self, hasher: Blake3) {
        self.file_hash = hasher;
    }

    pub fn transfer_id(&self) -> TransferId {
        self.transfer_id
    }

    async fn read_frame(&mut self) -> Result<Frame> {
        let len = Self::recv_msg(&mut self.recv_stream, &mut self.recv_buffer).await?;
        let mut payload = BytesMut::zeroed(MAX_NOISE_MESSAGE_LENGTH);
        let len = self
            .noise
            .read_message(&self.recv_buffer[0..len], &mut payload)?;
        let frame = Frame::read_from_frame(payload.split_to(len).into()).await?;

        Ok(frame)
    }

    async fn write_frame(&mut self, frame: Frame) -> Result<()> {
        let serialized_frame = frame.to_bytes();

        let len = self
            .noise
            .write_message(&serialized_frame, &mut self.send_buffer)?;

        Self::send_msg(&mut self.send_stream, &mut self.send_buffer[0..len]).await?;
        Ok(())
    }

    pub async fn send_ready(&mut self, seek: u64) -> Result<()> {
        self.write_frame(Frame::PleaseSendFile(seek)).await?;

        Ok(())
    }

    pub async fn wait_for_ready(&mut self) -> Result<u64> {
        match self.read_frame().await? {
            Frame::PleaseSendFile(seek) => Ok(seek),
            _ => panic!("unexpected response from sender"),
        }
    }

    pub async fn get_file_metadata(&mut self) -> Result<FlapFileMetadata> {
        match self.read_frame().await? {
            Frame::IWillSendThisFile(metadata) => Ok(metadata),
            _ => panic!("unexpected response from sender"),
        }
    }

    pub async fn send_file_metadata(&mut self, metadata: FlapFileMetadata) -> Result<()> {
        self.write_frame(Frame::IWillSendThisFile(metadata)).await?;

        Ok(())
    }

    pub async fn recv_next_file_block(&mut self, file: &mut File) -> Result<usize> {
        match self.read_frame().await? {
            Frame::FileData(file_data) => {
                self.file_hash.update_hasher(&file_data);
                match file.write_all(&file_data).await {
                    Err(e) => {
                        if e.kind() == ErrorKind::UnexpectedEof {
                            // We assume only one EOF per file, located
                            // at the end of the file. Therefore this is the
                            // EOF of the file that was sent to us.
                            // At this point the entire file should have been written.
                            file.flush().await?;

                            Ok(0)
                        } else {
                            // Other, actual I/O error.
                            Err(Error::FileIoError(e))
                        }
                    }
                    _ => {
                        file.flush().await?;

                        Ok(file_data.len())
                    }
                }
            }
            Frame::TransferComplete(sender_file_hash) => {
                let our_file_hash = self.file_hash.finalize_hash();
                println!("{}", base64ct::Base64::encode_string(&our_file_hash));
                file.sync_all().await?;

                if sender_file_hash != our_file_hash {
                    Err(Error::InvalidBlake3Hash)
                } else {
                    println!("File received successfully");

                    Ok(0)
                }
            }
            _ => unreachable!("file decryption shouldn't contain other types of frames"),
        }
    }

    pub async fn send_next_file_block(
        &mut self,
        file: &mut File,
        file_buf: &mut BytesMut,
    ) -> Result<usize> {
        let bytes_read = file.read(file_buf).await?;

        if bytes_read == 0 {
            let final_file_hash = self.file_hash.finalize_hash();

            self.write_frame(Frame::TransferComplete(final_file_hash))
                .await?;
            self.send_stream.finish()?;

            Ok(0)
        } else {
            self.file_hash.update_hasher(&file_buf);
            self.write_frame(Frame::FileData(file_buf.clone().freeze()))
                .await?;

            Ok(bytes_read)
        }
    }
}

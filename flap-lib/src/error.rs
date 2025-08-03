use thiserror::Error;

pub type Result<T> = core::prelude::v1::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown error")]
    Unknown,
    #[error("P2P accept error")]
    AcceptError(#[from] iroh::protocol::AcceptError),
    #[error("P2P connection error")]
    ConnectionError(#[from] iroh::endpoint::ConnectionError),
    #[error("P2P bind error")]
    BindError(#[from] iroh::endpoint::BindError),
    #[error("P2P connect error")]
    ConnectError(#[from] iroh::endpoint::ConnectError),
    #[error("P2P write error")]
    WriteError(#[from] iroh::endpoint::WriteError),
    #[error("P2P receive error")]
    ReadToEndError(#[from] iroh::endpoint::ReadToEndError),
    #[error("P2P connection closed")]
    ClosedStream(#[from] iroh::endpoint::ClosedStream),
    #[error("P2P file manager request error")]
    BlobRequestError(#[from] iroh_blobs::api::RequestError),
    #[error("Could not retrieve file from store")]
    ExportError(#[from] iroh_blobs::api::ExportBaoError),
    #[error("Could not decrypt file. Encryption key is likely invalid")]
    FileEncryptionError(#[from] aead::Error),
    #[error("Could not read ticket. Ticket is invalid")]
    TicketParseError,
    #[error("Could not read ticket's master key. Master key is invalid")]
    MasterKeyParseError,
    #[error("Could not read file. This is either a filesystem error or a lack of permission")]
    FileReadError,
}

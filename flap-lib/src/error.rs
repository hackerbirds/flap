use thiserror::Error;

pub type Result<T> = core::prelude::v1::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown error")]
    Unknown,
    #[error("frame serialization error")]
    SerializationError,
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
    ReadError(#[from] iroh::endpoint::ReadError),
    #[error("P2P receive error")]
    ReadExactError(#[from] iroh::endpoint::ReadExactError),
    #[error("P2P connection closed")]
    ClosedStream(#[from] iroh::endpoint::ClosedStream),
    #[error("Could not prepare file to encrypt")]
    MpscSendError,
    #[error("Could not encrypt/decrypt file. Encryption key or nonce is likely invalid.")]
    AeadError(#[from] aead::Error),
    #[error("An error occured in the encryption stream")]
    SnowError(#[from] snow::Error),
    #[error("Could not read ticket. Ticket is invalid")]
    TicketParseError,
    #[error("Could not read ticket's master key. Master key is invalid")]
    MasterKeyParseError,
    #[error("Could not read file. This is either a filesystem error or a lack of permission")]
    FileReadError,
    #[error("File has already been sent.")]
    FileAlreadyAdded,
    #[error("Filesystem IO error")]
    FileIoError(#[from] std::io::Error),
}

use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparingFileEvent {
    pub file_transfer_id: Vec<u8>,
    pub metadata: FileMetadata,
    pub sending: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub file_name: String,
    pub expected_file_size: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferUpdateEvent {
    pub file_transfer_id: Vec<u8>,
    pub bytes_downloaded: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferCompleteEvent {
    pub file_transfer_id: Vec<u8>,
}

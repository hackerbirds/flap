use std::{fs::File, io::Cursor};

/// Archive format used to keep metadata/file info when sharing
pub struct Tar {
    in_memory: Cursor<Vec<u8>>,
}

impl Tar {
    pub fn new(file_path: &str, mut file: File) -> Self {
        let cursor = Cursor::new(Vec::new());
        let mut builder = tar::Builder::new(cursor);

        builder.append_file(file_path, &mut file).unwrap();
        let archive = builder.into_inner().unwrap();

        Self { in_memory: archive }
    }
}

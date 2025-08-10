use std::{io::ErrorKind, path::PathBuf};

use tokio::fs::{DirBuilder, File};

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct FileSaver {
    /// The directory in which received files are written to.
    download_dir: PathBuf,
}

impl FileSaver {
    pub async fn new() -> Self {
        let download_dir = dirs::download_dir()
            .expect("download exists because Flap is used on a supported OS")
            .join("Flap Downloads");

        match DirBuilder::new().create(&download_dir).await {
            Ok(_) => {}
            Err(err) => match err.kind() {
                ErrorKind::AlreadyExists => {}
                _ => {
                    panic!(
                        "Flap does not have filesystem permissions, or there is a file at the path, or the filesystem returned a critical error"
                    )
                }
            },
        }

        Self { download_dir }
    }

    pub async fn prepare_file(&self, file_name: &str) -> Result<File> {
        let file_path = self.download_dir.join(file_name);
        let file: File = File::create_new(file_path).await?;

        Ok(file)
    }
}

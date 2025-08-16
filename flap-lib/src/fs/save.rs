use std::{
    io::{ErrorKind, SeekFrom},
    path::PathBuf,
};

use tokio::{
    fs::{self, DirBuilder, File},
    io::AsyncSeekExt,
};

use crate::{
    crypto::blake3::Blake3,
    error::{Error, Result},
    fs::metadata::FlapFileMetadata,
};

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

    pub async fn prepare_file(
        &self,
        metadata: &FlapFileMetadata,
    ) -> Result<(File, u64, Option<Blake3>)> {
        let mut file_name = metadata.file_name.clone();
        file_name.push_str(".flap");

        let file_path = self.download_dir.join(file_name);
        match File::create_new(&file_path).await {
            Ok(file) => Ok((file, 0, None)),
            Err(e) => {
                if matches!(e.kind(), ErrorKind::AlreadyExists) {
                    // Open with seek
                    let mut file = File::options()
                        .append(true)
                        .read(true)
                        .open(file_path)
                        .await?;
                    let file_len = file.metadata().await?.len();
                    let hasher = Blake3::partial_hash(&mut file, None).await?;

                    file.seek(SeekFrom::Start(file_len)).await?;

                    Ok((file, file_len, Some(hasher)))
                } else {
                    Err(Error::FileIoError(e))
                }
            }
        }
    }

    pub async fn finish_file(&self, metadata: &FlapFileMetadata) -> Result<()> {
        let file_name = metadata.file_name.clone();
        let mut file_name_with_flap_ext = metadata.file_name.clone();
        file_name_with_flap_ext.push_str(".flap");

        let file_path = self.download_dir.join(file_name);
        let file_path_with_ext = self.download_dir.join(file_name_with_flap_ext);

        fs::rename(file_path_with_ext, file_path).await?;

        Ok(())
    }
}

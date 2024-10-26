use std::{fs::OpenOptions, io::Write, path::Path};

use serde::{Deserialize, Serialize};

use crate::file2dl::File2Dl;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetaData {
    pub link: String,
    pub name_on_disk: String,
    pub url_name: String,
    pub content_length: usize,
    pub range_support: bool,
}

pub fn init_metadata(f: &File2Dl, dl_path: &str) -> Result<(), std::io::Error> {
    let meta_filename = format!("{}.metadl", &f.name_on_disk);
    let path = Path::new(dl_path).join(meta_filename);
    if !path.exists() {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        let meta_data = MetaData {
            link: f.url.link.clone(),
            name_on_disk: f.name_on_disk.clone(),
            url_name: f.url.filename.clone(),
            content_length: f.url.content_length,
            range_support: f.url.range_support,
        };
        let stringified = serde_json::to_string(&meta_data)?;
        file.write_all(stringified.as_bytes())?;
    }

    Ok(())
}

use std::{
    fs::{create_dir, metadata, read_dir, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::atomic::AtomicUsize,
    time::Duration,
};

use crate::{
    errors::{File2DlError, UrlError},
    metadata::{init_metadata, MetaData},
    url::Url,
};
use futures::StreamExt;
use reqwest::{header::RANGE, Client, ClientBuilder, Error, Response};
use std::sync::atomic::Ordering::Relaxed;
use tokio::sync::watch::{self, channel, Sender};

#[derive(Debug)]
pub struct File2Dl {
    pub url: Url,
    pub name_on_disk: String,
    pub dl_dir: String,
    pub size_on_disk: AtomicUsize,
    running: Sender<bool>,
    pub complete: bool,
}

impl File2Dl {
    pub async fn new(link: &str, download_path: &str) -> Result<Self, UrlError> {
        let url = Url::new(link).await?;
        if !Path::new(download_path).exists() {
            create_dir(download_path)?;
        }
        let name_on_disk = generate_name_on_disk(&url.filename, download_path)?;
        let (running, _) = watch::channel(false);
        let complete = false;
        let dl_dir = download_path.to_string();
        Ok(Self {
            url,
            name_on_disk,
            dl_dir,
            size_on_disk: AtomicUsize::new(0),
            running,
            complete,
        })
    }
    pub fn switch_status(&self) -> Result<(), File2DlError> {
        let rx = self.running.subscribe();
        let status = *rx.borrow();
        self.running.send(!status)?;
        Ok(())
    }
    pub async fn single_thread_dl(&mut self) -> Result<(), File2DlError> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(7))
            .build()?;
        //initialize the request based on the range support
        let res = init_res(self, &client).await?;
        //initialize metadata that will help in resume mechanism
        init_metadata(self, &self.dl_dir)?;
        let mut stream = res.bytes_stream();
        let full_path = Path::new(&self.dl_dir).join(&self.name_on_disk);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .truncate(false)
            .open(full_path)?;
        let mut rx = self.running.subscribe();
        while let Some(packed_chunk) = stream.next().await {
            rx.wait_for(|running| *running).await?;
            let chunk = packed_chunk?;
            file.write_all(&chunk)?;
            self.size_on_disk
                .fetch_add(chunk.len(), std::sync::atomic::Ordering::Relaxed);
        }
        self.complete = true;
        Ok(())
    }
    pub fn from(dir: &str) -> Result<Vec<File2Dl>, std::io::Error> {
        get_metadata_files(dir)?
            .into_iter()
            .map(|entry| {
                let m_data: MetaData = {
                    let path = Path::new(dir).join(&entry);
                    let mut buf = String::new();
                    File::open(&path)?.read_to_string(&mut buf)?;
                    serde_json::from_str(&buf)?
                };
                let size_on_disk = {
                    let file_path = Path::new(dir).join(&m_data.name_on_disk);
                    get_file_size(&file_path)?
                };

                let f2dl = {
                    let url = Url {
                        link: m_data.link,
                        filename: m_data.url_name,
                        content_length: m_data.content_length,
                        range_support: m_data.range_support,
                    };
                    let (tx, _) = channel(false);
                    let name_on_disk = {
                        if m_data.range_support {
                            m_data.name_on_disk
                        } else {
                            generate_name_on_disk(&m_data.name_on_disk, dir)?
                        }
                    };
                    File2Dl {
                        url,
                        dl_dir: dir.to_string(),
                        name_on_disk,
                        size_on_disk: AtomicUsize::new(size_on_disk),
                        running: tx,
                        complete: size_on_disk == m_data.content_length,
                    }
                };
                Ok(f2dl)
            })
            .collect()
    }
}

fn generate_name_on_disk(init: &str, download_path: &str) -> Result<String, std::io::Error> {
    let path = std::path::Path::new(download_path);
    let mut cpy = init.to_string();
    let mut idx = 1;
    let splits = init.rsplit('.').collect::<Vec<_>>();

    while path.join(cpy.clone()).exists() {
        cpy.clear();
        for i in (1..splits.len()).rev() {
            cpy.push_str(splits[i]);
            if i != 1 {
                cpy.push('.');
            }
        }
        let slice = format!("_{}.{}", idx, splits[0]);
        cpy.push_str(&slice);
        idx += 1;
    }

    Ok(cpy)
}
async fn init_res(f: &File2Dl, client: &Client) -> Result<Response, Error> {
    if f.url.range_support {
        return client
            .get(&f.url.link)
            .header(
                RANGE,
                format!(
                    "bytes={}-{}",
                    &f.size_on_disk.load(Relaxed),
                    &f.url.content_length
                ),
            )
            .send()
            .await;
    }
    client.get(&f.url.link).send().await
}

fn get_metadata_files(dir: &str) -> Result<Vec<String>, std::io::Error> {
    let collection = read_dir(dir)?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let file_name = e.file_name().to_str().unwrap_or_default().to_string();
                if file_name.ends_with(".metadl") {
                    Some(file_name.to_string())
                } else {
                    None
                }
            })
        })
        .collect::<Vec<String>>();
    Ok(collection)
}

fn get_file_size(path: &PathBuf) -> Result<usize, std::io::Error> {
    let metadata = metadata(path)?;
    let size = metadata.size();
    Ok(size as usize)
}

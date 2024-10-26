use std::time::Duration;

use crate::errors::UrlError;
use content_disposition::parse_content_disposition;
use regex::Regex;
use reqwest::{
    header::{HeaderMap, ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, RANGE},
    Client, ClientBuilder,
};

const URL_RE: &str = r#"(https:\/\/www\.|http:\/\/www\.|https:\/\/|http:\/\/)?[a-zA-Z0-9]{2,}(\.[a-zA-Z0-9]{2,})(\.[a-zA-Z0-9]{2,})?"#;
const FILENAME_RE: &str = r#"^[\w,\s-]+\.[A-Za-z]{3}$"#;

#[derive(Debug, Default)]
pub struct Url {
    pub link: String,
    pub filename: String,
    pub content_length: usize,
    pub range_support: bool,
}

impl Url {
    pub async fn new(link: &str) -> Result<Self, UrlError> {
        //self explanatory
        Self::is_valid_url(link)?;
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(7))
            .build()?;
        let headers = client.head(link).send().await?.headers().clone();
        //parses content length header else content length is 0
        let content_length = headers.content_length().unwrap_or_default();
        //parse name from content disposition header else parse from url else name is empty
        let filename = headers
            .content_dispo()
            .unwrap_or(parse_name_from_url(link).unwrap_or_default());
        //if header accept ranges exist then there is range support , else manually try a request with range
        let range_support = headers
            .accept_ranges()
            .unwrap_or(manual_range_test(&client, link).await);
        let link = link.to_owned();
        Ok(Self {
            link,
            filename,
            content_length,
            range_support,
        })
    }

    fn is_valid_url(link: &str) -> Result<(), UrlError> {
        //Self explanatory
        let re = Regex::new(URL_RE).expect("Invalid url regex");
        if !re.is_match(link) {
            return Err(super::errors::UrlError::InvalidUrl);
        }
        Ok(())
    }
}

pub trait ParseHeaders {
    fn content_length(&self) -> Option<usize>;
    fn accept_ranges(&self) -> Option<bool>;
    fn content_dispo(&self) -> Option<String>;
}

impl ParseHeaders for HeaderMap {
    //parses content length if its available
    fn content_length(&self) -> Option<usize> {
        self.get(CONTENT_LENGTH)
            .and_then(|length| length.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
    }

    fn accept_ranges(&self) -> Option<bool> {
        //checks range support through checking the header
        let range_header = self.get(ACCEPT_RANGES)?;
        if range_header.to_str().unwrap_or_default().trim() == "bytes" {
            return Some(true);
        }
        Some(false)
    }
    fn content_dispo(&self) -> Option<String> {
        //parses content disposition from header
        let content_dispo = self.get(CONTENT_DISPOSITION)?;
        let header_value = content_dispo.to_str().unwrap_or_default();
        let dis = parse_content_disposition(header_value);
        if let Some(fname) = dis.filename_full() {
            return Some(fname);
        }
        None
    }
}

async fn manual_range_test(client: &Client, link: &str) -> bool {
    //test range support through sending a ranged request (fallback for the header parse)
    match client.get(link).header(RANGE, "bytes=0-1").send().await {
        Ok(res) => res.bytes().await.map_or(false, |bytes| bytes.len() == 1),
        Err(_) => false,
    }
}

fn parse_name_from_url(link: &str) -> Option<String> {
    //self explanatory
    let splits = link.split('/');
    let re = Regex::new(FILENAME_RE).expect("Invalid filename regex");
    let last = splits.last()?;
    if re.is_match(last) {
        return Some(last.to_string());
    }
    None
}

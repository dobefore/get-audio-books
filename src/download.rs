use crate::error::{ApplicationError, Result};
use crate::site::Down;
use futures::future::join_all;
use futures::StreamExt;
use indicatif::ProgressBar;
use indicatif::{MultiProgress, ProgressState, ProgressStyle};
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use reqwest::IntoUrl;
use std::env;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time::Instant;
use std::{cmp::min, fmt::Write};
use tokio::fs::{remove_file, File};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
#[derive(Debug)]
pub struct Config {
    pub task_num: u64,
    pub uri: String,
    // pub path: String,
    pub file_path: PathBuf,
}
impl Down for Config {}
impl Config {
    pub fn new(task_num: u64, uri: &str, file_path: &Path) -> Self {
        Self {
            task_num,
            uri: uri.to_string(),
            file_path: file_path.to_owned(),
        }
    }
}

pub async fn check_request_range<U: IntoUrl>(url: U) -> Result<(bool, u64)> {
    let mut range = false;
    let req = reqwest::Client::new().head(url);
    let rep = req.send().await?;
    if !rep.status().is_success() {
        return Err(ApplicationError::ValueNotFound("request error".into()));
    }
    let headers = rep.headers();
    if headers
        .get(ACCEPT_RANGES)
        .map(|val| (val.to_str().ok()?.eq("bytes")).then(|| ()))
        .flatten()
        .is_some()
    {
        range = true;
    }
    let length = headers
        .get(CONTENT_LENGTH)
        .map(|val| val.to_str().ok())
        .flatten()
        .map(|val| val.parse().ok())
        .flatten()
        .ok_or(ApplicationError::ValueNotFound("get length fail".into()))?;
    Ok((range, length))
}

async fn download<U: IntoUrl>(
    url: U,
    (mut start, end): (u64, u64),
    is_partial: bool,
    file: Arc<Mutex<File>>,
) -> Result<()> {
    let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";

    let req = reqwest::ClientBuilder::new()
        .user_agent(pc)
        .build()?
        .get(url);
    let req = if is_partial {
        if end == u64::MAX {
            // request bytes from start to the end of file
            req.header(RANGE, format!("bytes={}-{}", start, ""))
        } else {
            req.header(RANGE, format!("bytes={}-{}", start, end))
        }
    } else {
        req
    };
    let rep = req.send().await?;
    if !rep.status().is_success() {
        return Err(ApplicationError::ValueNotFound("request error".into()));
    }

    let mut stream = rep.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let mut chunk = chunk?;
        let mut file = file.lock().await;
        file.seek(SeekFrom::Start(start)).await?;
        start += chunk.len() as u64;
        file.write_all_buf(&mut chunk).await?;
    }

    Ok(())
}
pub async fn new_run<U: IntoUrl, P: AsRef<Path>>(url: U, path: P, task_num: u64) -> Result<()> {
    let url = url.into_url()?;
    let mut handles = vec![];
    let (range, length) = check_request_range(url.clone()).await?;
    // if the size of local file which has the the name is identical to file which is downloading,skip
    let meta = tokio::fs::metadata(&path).await?;
    let size = meta.len();
    if size == length {
        return Ok(());
    }

    let file = Arc::new(Mutex::new(tokio::fs::File::create(&path).await?));
    // accept range field,
    let is_error = if range {
        // assume source= vec[1,2,3,4,5], source.len= 5 bytes
        // l/n : 5/3, 4/2
        let task_length = length / task_num;
        for i in 0..(task_num - 1) {
            // 线程数必须大于等于1
            let file = Arc::clone(&file);
            handles.push(tokio::spawn(download(
                url.clone(),
                (task_length * i, task_length * (i + 1) - 1),
                true,
                file,
            )));
        }
        {
            let file = Arc::clone(&file);
            handles.push(tokio::spawn(download(
                url.clone(),
                (task_length * (task_num - 1), u64::MAX),
                true,
                file,
            )));
        }

        let ret = join_all(handles).await;
        drop(file);
        ret.into_iter().flatten().any(|n| n.is_err())
    } else {
        download(url.clone(), (0, length - 1), false, file)
            .await
            .is_err()
    };
    if is_error {
        remove_file(&path).await?;
        Err(ApplicationError::ValueNotFound(
            "download file error".into(),
        ))
    } else {
        Ok(())
    }
}
pub async fn down(task_num: u64, uri: String, file_path: PathBuf) -> Option<()> {
    let config = Config::new(task_num, &uri, &file_path);
    // let now = Instant::now();
    if new_run(&config.uri, &file_path, config.task_num)
        .await
        .is_ok()
    {
        Some(())
    } else {
        None
    }
}
#[test]
fn test_down() {
    use tokio::runtime::Runtime;
    let rt = Runtime::new().unwrap();
    let url =
        "https://etc.usf.edu/lit2go/audio/mp3/the-adventures-of-huckleberry-finn-001-notice.97.mp3";

    let r = rt.block_on(down(10, url.to_string(), "./c.mp3".into()));
    if let Some(_) = r {
        println!("done");
    }
}
#[test]
fn test_content_range_length() {
    use tokio::runtime::Runtime;
    let rt = Runtime::new().unwrap();
    // use runtime instead of async
    let url =
        "https://etc.usf.edu/lit2go/audio/mp3/the-adventures-of-huckleberry-finn-001-notice.97.mp3";
    let req = reqwest::Client::new().head(url);
    let rep = rt.block_on(req.send()).unwrap();

    let headers = rep.headers();
    // return Some("bytes")
    let range = headers.get(ACCEPT_RANGES);
    assert_eq!(true, range.is_some());
    //   return Some("1315697") maybe in bytes size?
    let len = headers.get(CONTENT_LENGTH);
    assert_eq!(true, len.is_some())
}

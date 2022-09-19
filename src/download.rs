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
use std::time::{Duration, Instant};
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
    let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";

    let req = reqwest::ClientBuilder::new()
        .user_agent(pc)
        .build()?
        .head(url);
    // let req = reqwest::Client::new().head(url);
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
    bar: ProgressBar,time_limit: u64
) -> Result<()> {
    let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";

    let req = reqwest::ClientBuilder::new()
        .user_agent(pc)
        .build()?
        .get(url);
    let req = if is_partial {
        req.header(RANGE, format!("bytes={}-{}", start, end))
    } else {
        req
    };
    let rep = req.send().await?;
    if !rep.status().is_success() {
        return Err(ApplicationError::ValueNotFound("request error".into()));
    }
    let mut stream = rep.bytes_stream();
    let mut pos = 0;
    let sleep = tokio::time::sleep(Duration::from_secs(time_limit));
    tokio::pin!(sleep);
   let ret= loop {
        tokio::select! {
             ret = stream.next() =>{
                if let Some(chunk)=ret {
                    let mut chunk = chunk?;
                    let mut file = file.lock().await;
                    file.seek(SeekFrom::Start(start)).await?;
                    start += chunk.len() as u64;
                    pos += chunk.len() as u64;
                    file.write_all_buf(&mut chunk).await?;
                    bar.set_position(pos);
                }else {
                    break Ok(());
                }
                

            }
            _=&mut sleep =>{ break Err(ApplicationError::Download(format!("time exceed {} s",time_limit))) }
        } 
    };
   ret?;
   bar.finish();
    Ok(())
}
pub async fn new_run<U: IntoUrl, P: AsRef<Path>>(
    url: U,
    path: P,
    task_num: u64,
    mb: Arc<Mutex<MultiProgress>>,time_limit: u64
) -> Result<()> {
    let url = url.into_url()?;
    let mut handles = vec![];
    let (range, length) = check_request_range(url.clone()).await?;
    let file = Arc::new(Mutex::new(tokio::fs::File::create(&path).await?));
    // accept range field,
    // let pb = ProgressBar::new(end-start);
    let style= ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-");
    // mb.add(pb.clone();
    let is_error = if range {
        // let task_length = length / task_num;
        let range_vec = split_range(length, task_num);
        for (start, end) in range_vec {
            // 线程数必须大于等于1
            let bar = ProgressBar::new(end - start);
            bar.set_style(style.clone());
            mb.lock().await.add(bar.clone());
          

            let file = Arc::clone(&file);
            handles.push(tokio::spawn(download(
                url.clone(),
                (start, end),
                true,
                file,
                bar.clone(),time_limit.clone()
            )));
        }
        // for i in 0..(task_num - 1) {
        //     // 线程数必须大于等于1
        //     let file = Arc::clone(&file);
        //     handles.push(tokio::spawn(download(
        //         url.clone(),
        //         (task_length * i, task_length * (i + 1) - 1),
        //         true,
        //         file,
        //     )));
        // }
        // {
        //     let file = Arc::clone(&file);
        //     handles.push(tokio::spawn(download(
        //         url.clone(),
        //         (task_length * (task_num - 1), u64::MAX),
        //         true,
        //         file,
        //     )));
        // }

        let ret = join_all(handles).await;
        drop(file);
        ret.into_iter().flatten().any(|n| n.is_err())
    } else {
        let bar = ProgressBar::new(length - 1);
        bar.set_style(style.clone());
mb.lock().await.add(bar.clone());
        download(url.clone(), (0, length - 1), false, file, bar,time_limit.clone())
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
pub async fn down(
    time_limit: u64,
    task_num: u64,
    uri: String,
    file_path: PathBuf,
    mb: Arc<Mutex<MultiProgress>>,
) -> Result<()> {
    let config = Config::new(task_num, &uri, &file_path);
    // let now = Instant::now();
    // let sleep = tokio::time::sleep(Duration::from_secs(time_limit));
    // tokio::pin!(sleep);
    // tokio::select! {
    //         _= new_run(&config.uri, &file_path, config.task_num,mb.clone())=>{
    //         }
    // _=&mut sleep=>{
      
    //     mb.lock().await.clear().unwrap();
    //     // mb.lock().await;
    //  return   Err(ApplicationError::Download(format!("time exceed {} s",time_limit)))
    // }
    // _=tokio::signal::ctrl_c()=>{
    //    if file_path.exists(){
    //     tokio::fs::remove_file(file_path).await?;
    //    }
    // }
        // }
    new_run(&config.uri, &file_path, config.task_num,mb.clone(),time_limit).await?;

    // new_run(&config.uri, &file_path, config.task_num).await?;
    Ok(())
}

/// split bytes size into multi-range parts according to task_num
///
/// # example
/// ```
///  let v=  split_range(1024, 4);
///   println!("{:?}",v);
/// ```
/// output
/// `[(0, 255), (256, 511), (512, 767), (768, 1023)]`
fn split_range(bytes_size: u64, task_num: u64) -> Vec<(u64, u64)> {
    let mut parts = vec![];
    let length = bytes_size / task_num;
    (0..task_num - 1).for_each(|i| {
        let part = (i * length, i * length + length - 1);
        parts.push(part);
    });
    if !parts.is_empty() {
        let max = parts.last().as_ref().unwrap().1;
        parts.push((max + 1, bytes_size - 1))
    } else {
        // no for op is done,start form 0
        parts.push((0, bytes_size - 1))
    }
    parts
}
#[test]
fn test_range() {
    let v = split_range(1024, 4);
    println!("{:?}", v);
    split_range(5, 3);
}
#[test]
fn test_down() {
    // test download range,e.g. first download second part,at last down first part,use file vec
    // buffer to test.
    use tokio::runtime::Runtime;
    let mb = Arc::new(Mutex::new(MultiProgress::new()));
    let rt = Runtime::new().unwrap();
    let url =
        "https://etc.usf.edu/lit2go/audio/mp3/the-adventures-of-huckleberry-finn-001-notice.97.mp3";

    let r = rt.block_on(down(60, 3, url.to_string(), "./c.mp3".into(), mb));
    if let Err(e) = r {
        println!("{}", e);
    } else {
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

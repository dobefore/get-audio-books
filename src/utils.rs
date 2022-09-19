use std::path::Path;

use crate::error::Result;

pub(crate) async fn request_text(link: &str) -> Result<String> {
    let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
    let text = reqwest::ClientBuilder::new()
        .user_agent(pc)
        .build()?
        .get(link)
        .send()
        .await?
        .text()
        .await?;
    Ok(text)
}

/// compare file size with len of file stream.
pub(crate) async fn length_equal(path: &Path, length: u64) -> Result<bool> {
    // if the size of local file which has the the name is identical to file which is downloading,skip
    if path.exists() {
        let meta = tokio::fs::metadata(&path).await?;
        let size = meta.len();
        if size == length {
            return Ok(true);
        } else {
            Ok(false)
        }
    } else {
        Err(crate::error::ApplicationError::Download(
            "file path not exist".into(),
        ))
    }
}

/// compare file size with len of file stream.
pub(crate) async fn pdf_length_equal(path: &Path, length: u64) -> Result<bool> {
    // if the size of local file which has the the name is identical to file which is downloading,skip
    if path.exists() {
         if path
        .extension()
        .as_ref()
        .unwrap()
        .to_str()
        .as_ref()
        .unwrap()
        .to_owned()
        == "pdf" {
            let meta = tokio::fs::metadata(&path).await?;
            let size = meta.len();
            if size == length {
                return Ok(true);
            } else {
                Ok(false)
            }
        }else {
            Ok(false)
        }
    
        
    } else {
       Ok(false)
    }
}

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

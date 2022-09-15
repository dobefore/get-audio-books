use crate::error::Result;
use std::{
    io::{BufRead, Read, Write},
    path::Path,
    sync::Arc,
};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};
pub(crate) fn open_as_append(fname: &Path) -> Result<std::fs::File> {
    Ok(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(fname)?)
}
pub(crate) fn open_as_read(fname: &Path) -> Result<std::fs::File> {
    Ok(std::fs::File::open(fname)?)
}
pub(crate) fn open_as_write(fname: &Path) -> Result<std::fs::File> {
    Ok(std::fs::File::create(fname)?)
}
pub(crate) fn read_linnes(file: &mut std::fs::File) -> Result<Vec<String>> {
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    Ok(buf
        .lines()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.trim().is_empty())
        .collect::<Vec<_>>())
}
/// add new line break at the end of contents
pub(crate) fn append_str(file: &mut std::fs::File, contents: &str) -> Result<()> {
    writeln!(file, "{}\n", contents)?;
    Ok(())
}
/// add new line break at the end of contents
pub(crate) async fn append_str_async(file: &mut tokio::fs::File, contents: &str) -> Result<()> {
    file.write(format!("{}\n", contents).as_bytes()).await?;
    Ok(())
}
pub(crate) async fn open_as_append_async(fname: &Path) -> Result<tokio::fs::File> {
    Ok(tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(fname)
        .await?)
}
/// add new line break at the end of contents
pub(crate) async fn append_str_async_co<P: AsRef<Path>>(file: P, contents: String) -> Result<()> {
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .await?;
    f.write(format!("{}\n", contents).as_bytes()).await?;
    Ok(())
}

/// remove \
fn rmeove_backslash(dir: &Path) -> std::io::Result<()> {
    for p in std::fs::read_dir(dir)? {
        let p = p?.path();
        // read to string,strip \
        let ori = std::fs::read_to_string(&p)?;
        let new = ori.replace(r"\", "");
        std::fs::write(&p, format!("\"{}\"", new))?;
    }
    Ok(())
}

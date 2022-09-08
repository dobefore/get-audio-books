use crate::error::Result;
use std::{
    io::{BufRead, Read, Write},
    path::Path,
};
use tokio::io::{AsyncWrite, AsyncWriteExt};
pub(crate) fn open_as_append(fname: &Path) -> Result<std::fs::File> {
    Ok(std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(fname)?)
}
pub(crate) fn open_as_write(fname: &Path) -> Result<std::fs::File> {
    Ok(std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(fname)?)
}
pub(crate) fn open_as_read(fname: &Path) -> Result<std::fs::File> {
    Ok(std::fs::File::open(fname)?)
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
pub(crate) fn append_str(file: &mut std::fs::File, contents: &str) -> Result<()> {
    writeln!(file, "{}", contents)?;
    Ok(())
}
pub(crate) async fn append_str_async(file: &mut tokio::fs::File, contents: &str) -> Result<()> {
    file.write(contents.as_bytes()).await?;
    Ok(())
}
pub(crate) async fn open_as_append_async(fname: &Path) -> Result<tokio::fs::File> {
    Ok(tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(fname)
        .await?)
}

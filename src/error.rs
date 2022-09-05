use std::{num, result};
use thiserror::Error;
pub type Result<T> = result::Result<T, ApplicationError>;
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("ParseInt error: {0}")]
    ParseInt(#[from] num::ParseIntError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("ParseHtml error {0}")]
    ParseHtmlSelector(String),
    #[error("JsonParse error {0}")]
    JsonParse(#[from] json::JsonError),
    #[error("JsonParse error {0}")]
    Regex(#[from] regex::Error),
    #[error("ValueNotFound error {0}")]
    ValueNotFound(String)
}

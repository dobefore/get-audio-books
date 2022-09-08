use audio::AudioBook;
use clap::Parser;
use error::ApplicationError;

use crate::parse_args::ArgConfig;
mod audio;
mod error;
mod fileops;
mod parse_args;
mod site;
#[tokio::main]
async fn main() -> Result<(), ApplicationError> {
    let c = ArgConfig::parse();
    AudioBook::new(&c.output).operate(c).await?;
    Ok(())
}

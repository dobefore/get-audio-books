use crate::{
    error::Result,
    parse_args::ArgConfig,
    site::{convert_to_site, PagePattern1, Sites},
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
pub struct AudioBook {
    output: PathBuf,
    /// names of websites,e.g. Lit2go
    websites: Option<Vec<String>>,
}

impl AudioBook {
    pub(crate) fn new(output: &Path) -> Self {
        Self {
            output: output.into(),
            websites: None,
        }
    }
    /// show names of websites in site.rs
    ///
    /// a macro parse site.rs to get all structs
    fn display_websites() {}
    pub(crate) fn download(&self) -> Result<()> {
        Ok(())
    }
    /// handle arguments passed in command line
    ///
    /// e.g. show all books of a certain web site
    pub(crate) async fn operate(&self, config: &ArgConfig) -> Result<()> {
        if config.websites {}
        if let Some(site) = config.down.as_ref() {
            // down all
            // input 5 more links
            PagePattern1::new(
               site.to_owned(),
            )
            .down()
            .await?;
        }
        Ok(())
    }
}

use crate::{
    error::Result,
    parse_args::ArgConfig,
    site::{Lit2Go, PagePattern1},
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

    /// handle arguments passed in command line
    ///
    /// e.g. show all books of a certain web site
    pub(crate) async fn operate(&self, config: ArgConfig) -> Result<()> {
        if let Some(site) = config.crawl.as_ref() {
            // down all
            // input 5 more links
            let link_file = config.output.join("link_file.txt");
            let lit2go_file = config.output.join("bgs.txt");
            let mut pg = PagePattern1::new(site.to_owned());
            let mut lg = Lit2Go::new();
            if config.write {
                if config.lit2go {
                    lg.write(&config.output).await?;
                } else {
                    pg.write(&link_file).await?;
                }
            }
            if config.down {
                if config.lit2go {
                    lg.down(&config.output).await?;
                } else {
                    pg.down(&link_file, config.output.as_ref()).await?;
                }
            }
        }
        Ok(())
    }
}

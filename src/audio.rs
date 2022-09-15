use crate::{error::Result, lit2go::Lit2Go, parse_args::ArgConfig, site::PagePattern1};
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
            if config.book_folders {
                // assume there is a folder ,named book, which contains a list of files whose contents are
                // the same as those crawled from the website.
                // SO extract book name info from these files.
                if config.lit2go {
                    lg.create_book_folders().await?;
                }
                pg.create_book_folder(&config.output).await?;
            }
            if let Some(para) = config.print_links {
                pg.print_links(&para, &config.output)?;
            }
        }
        Ok(())
    }
}

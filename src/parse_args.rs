use std::path::PathBuf;

use clap::{ Parser};
use crate::error::Result;
use crate::config::Config;
/// crawl audio books from some websites
#[derive(Parser, Debug)]
#[clap( version, long_about = None)]
pub struct ArgConfig {
 ///Sets a custom config file,ie -c ankisyncd.toml
 #[clap(short, long, value_parser, value_name("file"),default_value("./audibooks.toml"))]
 pub config: PathBuf,
    #[clap(short, long, default_value("./output"))]
    pub output: PathBuf,
    /// write crawled book links to local file
    #[clap(short, long, action)]
    pub write: bool,
    /// begin process of downloading 
    #[clap(short, long, action)]
    pub down: bool,
   
}

/// Get config from path (if specified) or default value,
pub(crate) fn config_from_arguments(arg: &ArgConfig) -> Result<Config> {
   let p= arg.config.as_path();
     Ok(Config::from_file(p)?)
    
    
}
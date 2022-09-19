use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use crate::error::Result;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config{
    pub(crate) from_site:String,
pub(crate) download:bool,
/// crawl and write book links to file
pub(crate) write:bool
}
impl Default for Config {
    fn default() -> Self {
        Config {
            from_site:"lit2go".into(),
            download:false,
            write:false
        }
    }
}
impl Config {
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut config_string = String::new();
        file.read_to_string(&mut config_string)?;
        let c = toml::from_str(&config_string)?;
        Ok(c)
    }
}
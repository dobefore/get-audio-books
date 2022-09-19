use crate::{error::Result, lit2go::Lit2Go, parse_args::{ArgConfig, config_from_arguments}, site::PagePattern1};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
pub struct AudioBook {
    output: PathBuf,
    /// names of websites,e.g. Lit2go
    websites: Option<Vec<String>>,
}
enum  Sites{
    Lit2Go,
}
fn to_sites(s:&str)->Option<Sites>{
match s {
    "lit2go"=>Some(Sites::Lit2Go),
    _=>None
}
}
impl Sites {
  async  fn down(&self,output: &Path) -> Result<()>{
        match self {
            Self::Lit2Go=>{
                let  lg = Lit2Go::new();
                lg.down(output).await?;
                
            }
        } 
        Ok(())
    }
  async  fn write(&self,output: &Path) -> Result<()>{
        match self {
            Self::Lit2Go=>{
                let mut lg = Lit2Go::new();
                lg.write(output).await?;
            }
        }
        Ok(())
    }
}
impl AudioBook {
    pub(crate) fn new(output: &Path) -> Self {
        Self {
            output: output.into(),
            websites: None,
        }
    }
    /// handle arguments passed in command line
    ///
    /// e.g. show all books of a certain web site
    pub(crate) async fn operate(&self, arg: ArgConfig) -> Result<()> {
        let c=config_from_arguments(&arg)?;
        let output=arg.output;
        let site=to_sites(&c.from_site);

        if c.write {
            if let Some(s) =site.as_ref() {
                s.write(&output).await?;
            }
        }
        if c.download {
            if let Some(s) =site.as_ref()  {
                s.down(&output).await?;
            }
        }
        Ok(())
    }
}

use std::path::PathBuf;

use clap::{Args, Parser};
/// crawl audio books from some websites
#[derive(Parser, Debug)]
#[clap( version, long_about = None)]
pub struct ArgConfig {
    // /// db path/location
    // #[clap(flatten)]
    // pub location: Loc,
    /// specify location where audio books are downloaded, e.g. -o ./audio create output dir as
    /// target folder in current dir.
    #[clap(short, long, default_value("./output"))]
    pub output: PathBuf,
    /// print out current names of all websites
    #[clap(short, long, action)]
    pub websites: bool,
    /// download all books from a root_page 
    #[clap(short, long, value_parser)]
    pub down: Option<String>,
    // /// refreesh/update database e.g. -r C: D:
    // #[clap(
    //     short,
    //     long,
    //     multiple_values(true),
    //     value_name("HDDpath"),
    //     value_parser
    // )]
    // pub refresh: Option<Vec<String>>,
    // /// print results queried from db if it's true
    // #[clap(short, long, action)]
    // pub print: bool,
    // /// copy a book by an index to a PC storage
    // #[clap(short, long,number_of_values(2),multiple_values(true),value_names(&["index", "pclocation"]))]
    // pub copy: Option<Vec<String>>,
    // /// erase all data form db
    // #[clap(short, long, action)]
    // pub erase: bool,
}

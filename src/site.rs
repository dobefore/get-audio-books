use crate::{
    error::{ApplicationError, Result},
    fileops::{
        append_str, append_str_async, open_as_append, open_as_append_async, open_as_read,
        open_as_write, read_linnes,
    },
};
use async_trait::async_trait;
use json;
use regex::{internal::Input, Regex};
use scraper::{element_ref::Select, ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::SeekFrom,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    result,
    str::Bytes,
};
use tokio::io::AsyncWriteExt;

pub(crate) fn strip_invalid_str(source: &str) -> String {
    let mut no_invalid = String::from(source);
    let invalid_characters = [":", "?", "\\", "|", "*", "/", "\"", "<", ">"];
    // let invalid_c=invalid_characters.get(0).as_ref().unwrap();
    for c in invalid_characters {
        if source.contains(c) {
            no_invalid = no_invalid.clone().replace(c, "");
        } else {
            no_invalid = no_invalid.clone();
        }
    }
    no_invalid
}
///https://freeaudiobooksonline.net/audiobook-list/recommended-audiobooks
#[derive(Default, Debug)]
pub(crate) struct PagePattern1 {
    root_link: String,
    bookpages1: Option<Vec<BookPage1>>,
}
async fn down_local(
    book_title: &str,
    part_link: Option<String>,
    part_title: &str,
    book_title_code: &str,
    output: &Path,
) -> Result<()> {
    // create dir with book_title as its name ,write each part to this folder
    // skip if dir exists
    if !output.exists() {
        tokio::fs::create_dir(&output).await?;
    }
    let error_log = output.join("error.log");
    let book_dir = output.join(book_title_code);
    if !book_dir.exists() {
        tokio::fs::create_dir(&book_dir).await?;
    }
    println!("downloading book {}", book_title);

    let fname = format!("{}.mp3", part_title);
    if part_link.is_none() {
        let mut f = open_as_append_async(&error_log).await?;
        f.write(
            format!(
                "book part link is missing,whhose book name,part title are {},{}",
                book_title, part_title
            )
            .as_bytes(),
        )
        .await?;
    } else {
        println!(
            "downloading book part {} from link {}",
            part_title,
            part_link.as_ref().unwrap()
        );
        let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
    }
    Ok(())
}
impl PagePattern1 {
    pub(crate) fn new(root_link: String) -> Self {
        Self {
            root_link,
            ..Default::default()
        }
    }
    async fn request_text(&self, link: &str) -> Result<String> {
        let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
        let text = reqwest::ClientBuilder::new()
            .user_agent(pc)
            .build()?
            .get(link)
            .send()
            .await?
            .text()
            .await?;
        Ok(text)
    }
    fn update_bookpages(&mut self, bgs: Vec<BookPage1>) -> Result<()> {
        self.bookpages1 = Some(bgs);
        Ok(())
    }
    async fn update_parts(&mut self) -> Result<()> {
        if let Some(bgs) = self.bookpages1.as_mut() {
            for bg in bgs.iter_mut() {
                bg.update_parts().await?;
            }
        };

        Ok(())
    }
    fn parse_html_frac(&self, html: &str, selector: &str) -> Result<BookPage1> {
        let fragment = Html::parse_fragment(html);
        let selector = match Selector::parse(selector) {
            Ok(s) => Ok(s),
            Err(_) => Err(ApplicationError::ParseHtmlSelector(format!(
                "parse {} element error",
                selector
            ))),
        };
        let ele = fragment.select(&selector?).next();
        if ele.is_none() {
            return Err(ApplicationError::ParseHtmlSelector(
                "选择的元素为空或不存在".into(),
            ));
        }
        let link = ele.as_ref().unwrap().value().attr("href");
        let title = ele.as_ref().unwrap().text().collect::<Vec<_>>();
        Ok(BookPage1::new(
            link.unwrap().to_string(),
            title.last().unwrap().to_string(),
        ))
    }

    /// parse html str by CSS selector to get elements we want
    fn parse_html_doc(&self, html: &str, selector: &str) -> Result<Vec<BookPage1>> {
        let document = Html::parse_document(html);
        let selector = match Selector::parse(selector) {
            Ok(s) => s,
            Err(_) => {
                return Err(ApplicationError::ParseHtmlSelector(format!(
                    "parse {} element error",
                    selector
                )))
            }
        };
        let mut bgs = vec![];
        if document
            .select(&selector.clone())
            .collect::<Vec<_>>()
            .is_empty()
        {
            return Err(ApplicationError::ParseHtmlSelector(
                "选择的元素为空或不存在".into(),
            ));
        }
        for element in document.select(&selector) {
            // println!("{:?}",element.value().attr("href"))
            let bg = self.parse_html_frac(&element.html(), "a")?;
            bgs.push(bg);
            // println!("{:?}",element.value())
        }
        Ok(bgs)
    }

    /// write crawled book links to local file `link_file`
    pub(crate) async fn write(&mut self, link_file: &Path) -> Result<()> {
        let text = self.request_text(&self.root_link).await?;
        let bgs = self.parse_html_doc(
            &text,
            r#"p[class="name product-title woocommerce-loop-product__title"]"#,
        )?;
        self.update_bookpages(bgs)?;
        self.update_parts().await?;
        let mut f = open_as_append(&link_file)?;
        if let Some(bgs) = self.bookpages1.as_ref() {
            let mut book_num = 0;
            for bg in bgs {
                if let Some(parts) = bg.parts.as_ref() {
                    for part in parts {
                        let lg = LinkPage::new(
                            bg.book_title.as_ref().map(|e| e.to_string()),
                            Some(part.audio_title.to_owned()),
                            part.audio_link.as_ref().map(|e| e.to_string()),
                            book_num.to_string(),
                        );

                        append_str(&mut f, &lg.to_string())?;
                    }
                    book_num += 1;
                }
            }
        }

        // test audio link valid.
        Ok(())
    }

    /// rreturn links grouped by its book name  
    fn get_link_info_from_files(&self, output: &Path) -> Result<Vec<Result<Vec<String>>>> {
        let b = output.join("book");
        let f = std::fs::read_dir(b)?;
        let lines = f
            .into_iter()
            .map(|f| -> Result<Vec<String>> {
                let p = f?.path();
                Ok(std::fs::read_to_string(p)?
                    .lines()
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect::<Vec<_>>())
            })
            .collect::<Vec<_>>();

        Ok(lines)
    }
    pub(crate) async fn create_book_folder(&self, output: &Path) -> Result<()> {
        // dir : ./output/bok_name
        // read alist of files
        let b = output.join("book");
        let book_dir = output.join("book_dir");
        let error_log = output.join("error.log");
        if !book_dir.exists() {
            fs::create_dir(&book_dir)?;
        }
        let f = std::fs::read_dir(b)?;
        let lines = f
            .into_iter()
            .map(|f| -> Result<Vec<String>> {
                let p = f?.path();
                Ok(std::fs::read_to_string(p)?
                    .lines()
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect())
            })
            .collect::<Vec<_>>();
        for l in lines {
            let ll = l?;
            for line in ll {
                let lg: LinkPage = line.into();
                // remove invalid chacracters from book name
                let book_title = strip_invalid_str(lg.book_title.as_ref().unwrap());
                println!("creating book dir {:?}", book_title);
                let dir_to_create = book_dir.join(book_title);
                if !dir_to_create.exists() {
                    tokio::fs::create_dir(&dir_to_create).await?;
                }
                // write part link to corresponding book folder
                let file_name = dir_to_create.join("part_link.txt");
                let mut f = open_as_append_async(&file_name).await?;
                // tokio::fs::remove_file(file_name).await?;
                if lg.part_link.is_none() {
                    // write part title and book name to log file
                    let mut ff = open_as_append_async(&error_log).await?;
                    append_str_async(
                        &mut ff,
                        &format!(
                            "book name : {} part title : {}",
                            lg.book_title.as_ref().unwrap(),
                            lg.part_tilte.as_ref().unwrap()
                        ),
                    )
                    .await?;
                } else {
                    append_str_async(&mut f, lg.part_link.as_ref().unwrap()).await?;
                }
            }
        }
        Ok(())
    }
    fn sort_book(&self, lgs: Vec<LinkPage>, idx: u16) -> Result<()> {
        // let mut idx=0;

        Ok(())
    }
    fn cmp(&self, idx: u16, lg: Vec<String>) -> Result<Option<Vec<String>>> {
        let mut v = vec![];
        for l in lg {
            let lgg: LinkPage = l.into();
            if lgg.book_title_code.parse::<u16>()? == idx {
                if let Some(lk) = lgg.part_link {
                    v.push(lk);
                }
            }
        }
        if v.is_empty() {
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }

    fn max_min_num(&self, lgs: Vec<String>) -> Result<u16> {
        let mut bn = vec![];
        for l in lgs {
            let lgg: LinkPage = l.into();
            bn.push(lgg.book_title_code.parse::<u16>()?)
        }
        let max = get_max(bn.clone());
        let min = get_min(bn);
        Ok(max)
    }
    pub(crate) fn print_links(&self, param: &str, output: &Path) -> Result<()> {
        // 4 files of lines
        let lgs = self.get_link_info_from_files(output)?;
        let mut file_num = 0;
        for file in lgs {
            let flines = file?;
            let mut n = 0;
            // make hasgmap
            file_num += 1;
        }
        if param == "*" {
            //    means print links of all books
        } else {
            // accept number string to index book
        }
        Ok(())
    }
    // split code into 2 parts: write links to local and download from local links
    pub(crate) async fn down(&self, link_file: &Path, output: &Path) -> Result<()> {
        // read file from link_file
        let mut f = open_as_read(link_file)?;
        let lines = read_linnes(&mut f)?;

        for i in lines {
            let lg: LinkPage = i.into();
            let output = output.to_owned().clone();
            let links = output.join("links.txt");
            let error_log = output.join("error_log.txt");

            let mut f = open_as_append(&links)?;
            let mut error = open_as_append(&error_log)?;

            if lg.part_link.is_none() {
                writeln!(
                    &mut error,
                    "{}",
                    format!(
                        "book title {},part title {}",
                        lg.book_title.clone().as_ref().unwrap(),
                        lg.part_link.as_ref().unwrap(),
                    )
                )?;
            } else {
                writeln!(&mut f, "{}", lg.part_link.as_ref().unwrap())?;
            }
            //  down_local(
            //         lg.book_title.clone().as_ref().unwrap(),
            //         lg.part_link,
            //         lg.part_tilte.as_ref().unwrap(),
            //         &lg.book_title_code,
            //        &output ,
            //     )
            //     .await?

            //    ;
        }

        Ok(())
    }
}

#[derive(Default, Debug)]
struct BookPage1 {
    book_link: Option<String>,
    book_title: Option<String>,
    parts: Option<Vec<Part>>,
}

impl BookPage1 {
    fn new(book_link: String, book_title: String) -> Self {
        Self {
            book_link: Some(book_link),
            book_title: Some(book_title),
            ..Default::default()
        }
    }
    async fn parse_html_frac(&self, html: &str, selector: &str) -> Result<String> {
        let fragment = Html::parse_fragment(html);
        let selector = match Selector::parse(selector) {
            Ok(s) => Ok(s),
            Err(_) => Err(ApplicationError::ParseHtmlSelector(format!(
                "parse {} element error",
                selector
            ))),
        };
        let ele = fragment.select(&selector?).next();
        let title = ele.as_ref().unwrap().text().collect::<Vec<_>>();
        Ok(title.last().unwrap().to_string())
    }
    fn json_parse(&self, json_str: &str) -> Result<Vec<Part>> {
        let mut parts = vec![];
        let mut json = json::parse(json_str)?;
        let tracks = json.remove("tracks");
        let v = tracks.to_string();
        let re = Regex::new(r"\{(.*?)\}\},")?;
        for cap in re.captures_iter(&v) {
            let cap_str = cap.get(0).unwrap().as_str();
            let mut js = json::parse(cap_str.strip_suffix(",").unwrap())?;
            let link = js.remove("src");
            let title = js.remove("title");
            let link = if link.is_empty() {
                Err(ApplicationError::ValueNotFound(
                    "empty value in link".into(),
                ))
            } else {
                Ok(link)
            };
            let link = match link {
                Ok(l) => Some(l),
                Err(_) => None,
            };
            let part = Part::new(link.map(|w| w.to_string()), title.to_string());
            parts.push(part);
        }

        Ok(parts)
    }
    /// parse html str by CSS selector to get elements we want
    async fn parse_html_doc(&self, html: &str, selector: &str) -> Result<Vec<Part>> {
        let document = Html::parse_document(html);
        let selector = match Selector::parse(selector) {
            Ok(s) => Ok(s),
            Err(_) => Err(ApplicationError::ParseHtmlSelector(format!(
                "parse {} element error",
                selector
            ))),
        };
        let element = document.select(&selector?).next();
        if element.is_none() {
            return Err(ApplicationError::ParseHtmlSelector(
                "选择的元素为空或不存在".into(),
            ));
        }
        let parts = self.json_parse(
            element
                .as_ref()
                .unwrap()
                .text()
                .collect::<Vec<_>>()
                .last()
                .unwrap(),
        )?;

        Ok(parts)
    }
    async fn request_text(&self, link: &str) -> Result<String> {
        let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
        let text = reqwest::ClientBuilder::new()
            .user_agent(pc)
            .build()?
            .get(link)
            .send()
            .await?
            .text()
            .await?;
        Ok(text)
    }
    async fn update_parts(&mut self) -> Result<()> {
        // request book page to get html
        println!(
            "updating parts from book link {:?}",
            self.book_link.as_ref()
        );

        let html = self.request_text(self.book_link.as_ref().unwrap()).await?;
        //  parse html to get audio lnk and title
        // let parts = self.parse_html_doc(&html, r#"div[class="wp-playlist-item wp-playlist-playing"]"#).await?;
        let parts = self
            .parse_html_doc(&html, r#"script[class="wp-playlist-script"]"#)
            .await?;
        // update parts by updating.
        self.parts = Some(parts);
        Ok(())
    }
}
#[derive(Default, Debug)]
pub(crate) struct Part {
    pub(crate) audio_link: Option<String>,
    pub(crate) audio_title: String,
}

impl Part {
    fn new(audio_link: Option<String>, audio_title: String) -> Self {
        Self {
            audio_link,
            audio_title,
        }
    }
}

pub(crate) trait Utils {
    type Item;
    fn count(&self, c: Option<Vec<Self::Item>>) -> u8 {
        if let Some(cc) = c {
            cc.len() as u8
        } else {
            0
        }
    }
}
#[async_trait]
pub(crate) trait Down {
    async fn request_builder(&self, url: &str) -> Result<reqwest::RequestBuilder> {
        let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
        Ok(reqwest::ClientBuilder::new()
            .user_agent(pc)
            .build()?
            .get(url))
    }
    async fn request_bytes(&self, link: &str) -> Result<Vec<u8>> {
        let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
        Ok(reqwest::ClientBuilder::new()
            .user_agent(pc)
            .build()?
            .get(link)
            .send()
            .await?
            .bytes()
            .await?
            .to_vec())
    }
    async fn request_text(&self, link: &str) -> Result<String> {
        let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
        let text = reqwest::ClientBuilder::new()
            .user_agent(pc)
            .build()?
            .get(link)
            .send()
            .await?
            .text()
            .await?;
        Ok(text)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub(crate) struct AudioLink {
    audio_link: Option<String>,
    /// contain mulit-line plain text string
    text: String,
}
unsafe impl Sync for AudioLink {}
impl AudioLink {
    pub(crate) fn new(audio_link: Option<String>, text: String) -> Self {
        Self { audio_link, text }
    }

    pub(crate) fn audio_link(&self) -> Option<String> {
        self.audio_link.as_ref().map(|e| e.to_string())
    }

    pub(crate) fn text(&self) -> String {
        self.text.to_string()
    }
}

#[derive(Debug, Clone)]
struct LinkPage {
    book_title_code: String,
    book_title: Option<String>,
    part_tilte: Option<String>,
    part_link: Option<String>,
}

impl LinkPage {
    fn new(
        book_title: Option<String>,
        part_tilte: Option<String>,
        part_link: Option<String>,
        book_title_code: String,
    ) -> Self {
        Self {
            book_title,
            part_tilte,
            part_link,
            book_title_code,
        }
    }
}
impl Display for LinkPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.part_link.is_none() {
            write!(
                f,
                "{}:::{}:::{}",
                self.book_title_code,
                self.book_title.as_ref().unwrap(),
                self.part_tilte.as_ref().unwrap()
            )
        } else {
            write!(
                f,
                "{}:::{}:::{}:::{}",
                self.book_title_code,
                self.book_title.as_ref().unwrap(),
                self.part_tilte.as_ref().unwrap(),
                self.part_link.as_ref().unwrap()
            )
        }
    }
}
impl From<String> for LinkPage {
    fn from(e: String) -> Self {
        // string format : booktitle:::parttitle:::partlink
        let s = e
            .split(":::")
            .collect::<Vec<_>>()
            .iter()
            .map(|el| el.to_string())
            .collect::<Vec<_>>();
        if s.len() == 3 {
            Self {
                book_title: s.get(1).map(|e| e.to_owned()),
                part_tilte: s.get(2).map(|e| e.to_owned()),
                part_link: None,
                book_title_code: s.get(0).as_ref().unwrap().to_string(),
            }
        } else {
            Self {
                book_title: s.get(1).map(|e| e.to_owned()),
                part_tilte: s.get(2).map(|e| e.to_owned()),
                part_link: s.last().map(|e| e.to_owned()),
                book_title_code: s.get(0).as_ref().unwrap().to_string(),
            }
        }
    }
}
fn get_max(ns: Vec<u16>) -> u16 {
    let mut max = ns.get(0).unwrap().to_owned();
    for i in ns.clone() {
        if max < i {
            max = i;
        } else {
            max = max;
        }
    }
    max
}
fn get_min(ns: Vec<u16>) -> u16 {
    let mut min = ns.get(0).unwrap().to_owned();
    for i in ns.clone() {
        if min > i {
            min = i;
        } else {
            min = min;
        }
    }
    min
}
#[test]
fn text_get_max() {
    let ns = [3, 2, 0, 1, 5, 5, 9, 19, 11];
    let m = get_max(ns.to_vec());
    let mi = get_min(ns.to_vec());
    assert_eq!(19, m);
    assert_eq!(0, mi);
}

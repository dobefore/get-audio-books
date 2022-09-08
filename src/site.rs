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
    fmt::Display,
    fs::File,
    io::SeekFrom,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    str::Bytes,
};
use tokio::io::AsyncWriteExt;
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
      
        // tokio::process::Command::new("curl").args(&[part_link.as_ref().unwrap(),"-o",book_dir.join(fname).to_str().unwrap(),]).status().await.expect("");
        // std::process::Command::new("curl").args(&[part_link.as_ref().unwrap(),"-o",book_dir.join(fname).to_str().unwrap(),"-s"]).status().expect("");

        // std::process::Command::new("curl").args(&[part_link.as_ref().unwrap(),"-o",book_dir.join(fname).to_str().unwrap(),"-s"]).status().expect("");
        //     let bytes = reqwest::blocking::ClientBuilder::new()
        //       .user_agent(pc)
        //       .build()?.get(part_link.as_ref().unwrap())
        //       .send()?.bytes()? ;

        // tokio::fs::write(book_dir.join(fname), bytes).await?;
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

/// links.txt
#[derive(Debug, Default, Serialize, Deserialize)]
struct Lit2GoLinks {
    links: Option<Vec<Lit2GoLink>>,
}

impl Lit2GoLinks {
    fn new(links: Option<Vec<Lit2GoLink>>) -> Self {
        Self { links }
    }
}
/// https://etc.usf.edu/lit2go/books/
#[derive(Debug, Default, Serialize, Deserialize)]
struct Lit2GoLink {
    book_title_code: Option<String>,
    book_title: Option<String>,
    /// chapter_name as the filename of audio and odf
    chapter_name: Option<String>,
    audio_link: Option<String>,
    /// multi-line string
    text: Option<String>,
}

impl Lit2GoLink {
    fn new(
        book_title_code: Option<String>,
        book_title: Option<String>,
        chapter_name: Option<String>,
        audio_link: Option<String>,
        text: Option<String>,
    ) -> Self {
        Self {
            book_title_code,
            book_title,
            chapter_name,
            audio_link,
            text,
        }
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Lit2Go {
    pub(crate) root_site: Option<String>,
      /// show how many books this website  will have 
      book_count:Option<u8>,
    bookpages: Option<Vec<BookPage>>,
}
impl Utils for Lit2Go {type Item = Option<Vec<BookPage>>;    }
impl Down for Lit2Go {}
impl Lit2Go {
    fn count_actual_books( &self,)->u8 {
        if let Some(bgs) =self.bookpages.as_ref()  {
            bgs.len() as u8
        }else {
            0
        }
    }
    fn equal_non_zero( &self,)->bool {
        let acctual=self.count_actual_books();
        if acctual!=0 {
            if acctual==self.book_count.unwrap() {
               true
            }else {
                false
            }  
        }else {
            false
        }
       
    }
     /// check whether chapters in each book are fully parsed by check count equal 
     fn check_audio_sanity_of_chapters( &self,)->bool {
        self.bookpages.as_ref().unwrap().iter().all(|e|e.chapters.as_ref().unwrap().iter().all(|c|c.audio_sanity()))
    
        }
    /// check whether chapters in each book are fully parsed by check count equal 
    fn check_chapter_sanity_of_books( &self,)->bool {
    self.bookpages.as_ref().unwrap().iter().all(|e|e.equal_non_zero())

    }
    /// chapter_name as the filename of audio and odf
    async fn down_local(
        &self,
        book_title_code: &str,
        book_title: &str,
        chapter_name: &str,
        audio_link: &str,
        pdf_link: &str,
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

        let audio_fname = format!("{}.mp3", chapter_name);
        let pdf_fname = format!("{}.pdf", chapter_name);

        println!(
            "downloading book chapter {} from link {}",
            chapter_name, audio_link
        );

        // tokio::process::Command::new("curl").args(&[part_link.as_ref().unwrap(),"-o",book_dir.join(fname).to_str().unwrap(),"-s"]).spawn().expect("");
        // std::process::Command::new("curl").args(&[part_link.as_ref().unwrap(),"-o",book_dir.join(fname).to_str().unwrap(),"-s"]).spawn().expect("");
        let audio_bytes = self.request_bytes(&audio_link).await?;
        let pdf_bytes = self.request_bytes(&pdf_link).await?;

        tokio::fs::write(book_dir.join(audio_fname), audio_bytes).await?;
        tokio::fs::write(book_dir.join(pdf_fname), pdf_bytes).await?;

        Ok(())
    }
    pub(crate) async fn down(&self, output: &Path) -> Result<()> {
        // read links from file via serde_json
        let links_file = output.join("links.txt");
        let f = open_as_read(&links_file)?;
        let reader = std::io::BufReader::new(f);
        let links: Lit2GoLinks = serde_json::from_reader(reader)?;
        // loop links to download to local async
        if let Some(links) = links.links.as_ref() {
            for link in links {
                let book_title_code = link.book_title_code.as_ref().unwrap();
                let book_title = link.book_title.as_ref().unwrap();
                let audio_link = link.audio_link.as_ref().unwrap();

                let chapter_name = link.chapter_name.as_ref().unwrap();

                let pdf_link = link.text.as_ref().unwrap();

                self.down_local(
                    book_title_code,
                    book_title,
                    chapter_name,
                    audio_link,
                    pdf_link,
                    output,
                )
                .await?;
            }
        }

        // in order to avoid filename naming error due to illegal characters like : ,we
        // have to remove/strip this from chapter name
        Ok(())
    }
    fn blank(&self) -> Result<()> {
        Ok(())
    }
    pub(crate) fn new() -> Self {
        Self {
            root_site: Some("https://etc.usf.edu/lit2go/books/".into()),
            ..Self::default()
        }
    }
    fn paese_book_page(
        &self,
        html: &str,
        selector: &str,
        sub_selector: &str,
    ) -> Result<Vec<BookPage>> {
        let htmls = parse_html_doc(&html, selector)?;
        let mut bgs = vec![];
        for html in htmls {
            let (link, title) = parse_html_frac(&html, sub_selector)?;
            let bg = BookPage::new(title, link.as_ref().unwrap().to_string());
            bgs.push(bg);
        }
        Ok(bgs)
    }
    /// write parsed links to local
    ///
    /// format: `book_title_code:::book_title:::chapter_title:::audio_link:::pdf_link`
    ///
    /// audio_link and pdf_link named after chapter_title ,notice remove :
    ///
    /// book page parse html figcaption[class="title"] -> parse frac `a`
    ///
    /// through inspecting some book site,find some chapters have no pdf link/element (e.g. PREFACE),
    /// even formal chapters are the same. so I crawl plain text instead of pdf link no matter where pdf link is present.
    /// And need to crawl a bigger html part while getting audio links,namely `div`,then get audio link from it,and
    /// get text from it too,if text and audio are both absent,log it.
    ///
    /// /// Some chapters have 3 item in audio row.
    ///
    /// # How to crawl audio and plain text
    ///
    /// due to deferences described above,I change to elements to be parsed.
    ///
    /// 1. crawl a bigger part of html including audio link and plain text, div[id="i_apologize_for_the_soup"]
    ///
    /// 2. parse audio and text , audio:tag "source" ,first matched ele,get attr "src".
    /// palin text(text is arranged in a vec of tag p) : parse tag p ,get text ,then join text (note \n,add if not)

    pub(crate) async fn write(&mut self, output: &Path) -> Result<()> {
        // parse book page,if bgs_file exist skip
        let lit2go_file = output.join("lit2go.txt");
        let error_log = output.join("error_log.txt");
        let lit2g_links = output.join("lit2g_links.txt");

        if lit2go_file.exists() {
            let f = open_as_read(&lit2go_file)?;
            let reader = std::io::BufReader::new(f);
            let lg: Lit2Go = serde_json::from_reader(reader)?;
            self.set_bookpages(Some(lg.bookpages().unwrap().to_vec()));
            if self.equal_non_zero(){
                return Err(ApplicationError::ValueNotEqual("book numbers are not equal".into()));
            }
            println!("loaded bookpages from file");
        } else {
            let html = self.request_text(self.root_site.as_ref().unwrap()).await?;
            // in the begining, total books should be counted by count elements.
            let bgs = self.paese_book_page(&html, r#"figcaption[class="title"]"#, "a")?;
           self.set_book_count(Some(bgs.len() as u8));
            self.set_bookpages(Some(bgs));
            println!("loaded bookpages from parsing crawling")
        }

        println!(" write litgo struct to local");

        let lg_str = serde_json::to_string(&self)?;
        std::fs::write(&lit2go_file, lg_str)?;

        // parse chapter pages of each book
        // if chapter is none ,it means chapters of each have not been parsed,need parsing
        // write to lit2go
        if !self.check_chapter_sanity_of_books(){
            // if encountered error,skip this book and log it
          match   self.update_chapters_of_each_book(&error_log).await {
              Ok(_)=>{},
              Err(_)=>{
                let lg_str = serde_json::to_string(&self)?;
                std::fs::write(&lit2go_file, lg_str)?;
                return Err(ApplicationError::UpdateLit2go("update_chapters_of_each_book".into()));
              }
          };
            let lg_str = serde_json::to_string(&self)?;
            std::fs::write(&lit2go_file, lg_str)?;
        }
        println!("update chapters done");
        // if audiolink is none ,it means audio of each has not been parsed,need parsing
        // write to lit2go
        if self.check_audio_sanity_of_chapters() {

            match self.update_audio_of_each_chapter(&error_log).await {
                Ok(_)=>{},
                Err(_)=>{
                  let lg_str = serde_json::to_string(&self)?;
                  std::fs::write(&lit2go_file, lg_str)?;
                  return Err(ApplicationError::UpdateLit2go("update_audio_of_each_chapter".into()));
                }
            };
            let lg_str = serde_json::to_string(&self)?;
            std::fs::write(&lit2go_file, lg_str)?;
        }
        println!("set audio done");
        let  lgl = open_as_write(&lit2g_links)?;
        let writer = std::io::BufWriter::new(lgl);
        self.write_local(writer)?;
        Ok(())
    }
    fn write_local(&self, writer: BufWriter<File>) -> Result<()> {
        let bgs = self.bookpages.as_ref();
        if let Some(bgs) = bgs {
            let mut book_num = 0;
            let mut lgl_vec = vec![];
            for bg in bgs {
                if let Some(chapters) = bg.chapters.as_ref() {
                    for chapter in chapters {
                        let audio_link = chapter.audio.as_ref().unwrap().audio_link();
                        let text = chapter.audio.as_ref().unwrap().text();
                        if audio_link.is_none() {
                            return Err(ApplicationError::ValueNotFound(
                                "write to local error , audio link not found".into(),
                            ));
                        }
                        let lgl = Lit2GoLink::new(
                            Some(book_num.to_string()),
                            Some(bg.book_name()),
                            Some(chapter.chapter_name()),
                            Some(audio_link.as_ref().unwrap().to_string()),
                            Some(text),
                        );
                        lgl_vec.push(lgl);
                    }
                    book_num += 1;
                }
            }
            let lgls = Lit2GoLinks::new(Some(lgl_vec));
            let s = serde_json::to_string(&lgls)?;
            serde_json::to_writer(writer, &s)?;
        }

        Ok(())
    }
    async fn update_audio_of_each_chapter(&mut self, error_file: &Path) -> Result<()> {
        // loop ops
        let mut f = open_as_append_async(error_file).await?;
        for bg in self.bookpages.as_mut().unwrap() {
             bg.update_audio().await?;
        }
        Ok(())
    }
    async fn update_chapters_of_each_book(&mut self, error_file: &Path) -> Result<()> {
        // loop ops
        // let mut f = open_as_append_async(error_file).await?;
        for bg in self.bookpages.as_mut().unwrap() {
             bg.update_chapters().await ?;
        }
        Ok(())
    }
    pub(crate) fn set_bookpages(&mut self, bookpages: Option<Vec<BookPage>>) {
        self.bookpages = bookpages;
    }

    pub(crate) fn bookpages(&self) -> Option<&Vec<BookPage>> {
        self.bookpages.as_ref()
    }
    pub(crate) fn last_bookpage(&self) -> &BookPage {
        self.bookpages.as_ref().unwrap().last().as_ref().unwrap()
    }

    pub(crate) fn set_book_count(&mut self, book_count: Option<u8>) {
        self.book_count = book_count;
    }
}

fn selector_parse_doc(html: &str, selector: &str) -> Result<(Html, Selector)> {
    let document = Html::parse_document(html);
    match Selector::parse(selector) {
        Ok(s) => return Ok((document, s)),
        Err(_) => {
            return Err(ApplicationError::ParseHtmlSelector(format!(
                "parse {} element error",
                selector
            )))
        }
    };
}
fn selector_parse_frac(html: &str, selector: &str) -> Result<(Html, Selector)> {
    let fragment = Html::parse_fragment(html);
    match Selector::parse(selector) {
        Ok(s) => Ok((fragment, s)),
        Err(_) => Err(ApplicationError::ParseHtmlSelector(format!(
            "parse {} element error",
            selector
        ))),
    }
}
/// return a pair of link and title
fn parse_html_frac(html: &str, selector: &str) -> Result<(Option<String>, String)> {
    let (document, selector) = selector_parse_frac(html, &selector)?;
    let mut elements = document.select(&selector);

    // assume there is only one ele
    if let Some(e) = elements.next() {
        let link = e.value().attr("href");
        let title = e.text();
        return Ok((
            link.map(|e| e.to_string()),
            title
                .collect::<Vec<_>>()
                .last()
                .as_ref()
                .unwrap()
                .to_string(),
        ));
    } else {
        println!("{}", format!("html {}, selector ", html,));
        return Err(ApplicationError::ParseHtmlSelector(
            "选择的元素为空或不存在".into(),
        ));
    }
}
/// return html strings of matched elements
fn parse_html_doc(html: &str, selector: &str) -> Result<Vec<String>> {
    let (document, selector) = selector_parse_doc(html, selector)?;
    let elements = document.select(&selector).collect::<Vec<_>>();
    if elements.is_empty() {
        println!("parse_html_doc {}", format!("html {}, selector ", html,));

        return Err(ApplicationError::ParseHtmlSelector(
            "选择的元素为空或不存在".into(),
        ));
    }
    let mut htmls = vec![];
    for e in elements {
        htmls.push(e.html());
    }
    Ok(htmls)
}
trait Utils {
    type Item;
    fn count(&self,c:Option<Vec<Self::Item>>)->u8 {
        if let Some(cc) =c  {
            cc.len() as u8
        }else {
            0
        }
    }
}
#[async_trait]
trait Down {
    async fn request_bytes(&self, link: &str) -> Result<Vec<u8>> {
        let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
        Ok(reqwest::blocking::ClientBuilder::new()
            .user_agent(pc)
            .build()?
            .get(link)
            .send()?
            .bytes()?
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
pub(crate) struct BookPage {
    book_name: String,
    book_link: String,
    /// show how many chapters this book will have 
    chapter_count:Option<u8>,
    chapters: Option<Vec<Chapter>>,
}
impl Utils for BookPage {type Item =Option<Vec<Chapter>>;    }

impl Down for BookPage {}
impl BookPage {
    fn new(book_name: String, book_link: String) -> Self {
        Self {
            book_name,
            book_link,
            ..Default::default()
        }
    }
   
    fn count_actual_books( &self,)->u8 {
        if let Some(bgs) =self.chapters.as_ref()  {
            bgs.len() as u8
        }else {
            0
        }
    }
    fn equal_non_zero( &self,)->bool {
        let acctual=self.count_actual_books();
        if acctual!=0 {
            if acctual==self.chapter_count.unwrap() {
               true
            }else {
                false
            }  
        }else {
            false
        }
       
    }
    /// enter each book page ,parse html `dt` -> parse frac `a` to get chapters
    ///
    fn paese_chapter_page(
        &self,
        html: &str,
        selector: &str,
        sub_selector: &str,
    ) -> Result<Vec<Chapter>> {
        let htmls = parse_html_doc(&html, selector)?;
        let mut chps = vec![];
        println!("paese_chapter_page");
        for html in htmls {
            println!("chapter {}", html.clone());
            let (link, title) = parse_html_frac(&html, sub_selector)?;
            let chap = Chapter::new(title, link.as_ref().unwrap().to_string());
            chps.push(chap);
        }
        Ok(chps)
    }
    async fn update_chapters(&mut self) -> Result<()> {
        // parse html of book link to get chapters
        // skip if chapter sanity test is ok
        if self.equal_non_zero() {
            return Ok(());
        }
        let html = self.request_text(self.book_link.as_ref()).await?;
        match self.paese_chapter_page(&html, "dt", "a") {
            Ok(cs) => {
                self.set_chapter_count(Some(cs.len() as u8));
                self.set_chapters(Some(cs))
            },
            Err(_) => {
                let mut f = open_as_append_async("error.log".as_ref()).await?;
                f.write(self.book_name.as_bytes()).await?;
                return Err(ApplicationError::UpdateLit2go("update_chapters".into()));
            }
        };
        Ok(())
    }
    async fn update_audio(&mut self) -> Result<()> {
        if let Some(cs) = self.chapters.as_mut() {
            for c in cs {
        // skip if audio link exists 
                if c.audio_sanity() {
                    continue;
                }
                c.update_audio().await?;
            }
        }

        Ok(())
    }
    fn book_name(&self) -> String {
        self.book_name.to_string()
    }

    pub(crate) fn set_chapters(&mut self, chapters: Option<Vec<Chapter>>) {
        self.chapters = chapters;
    }
    pub(crate) fn last_chapter(&self) -> &Chapter {
        self.chapters.as_ref().unwrap().last().as_ref().unwrap()
    }
    pub(crate) fn chapters(&self) -> Option<&Vec<Chapter>> {
        self.chapters.as_ref()
    }

    pub(crate) fn set_chapter_count(&mut self, chapter_count: Option<u8>) {
        self.chapter_count = chapter_count;
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub(crate) struct Chapter {
    chapter_name: String,
    capter_link: String,
    /// one chapter has one audio
    pub(crate) audio: Option<AudioLink>,
}
impl Down for Chapter {}
impl Chapter {
    fn new(chapter_name: String, capter_link: String) -> Self {
        Self {
            chapter_name,
            capter_link,
            audio: None,
        }
    }
    /// return false if [`AudioLink`] is none or `audio_link` is none
    fn audio_sanity(&self,)->bool{
if self.audio.is_none() {
    false
}else {
if    self.audio.as_ref().unwrap().audio_link().is_none()
{
    false
}else {
    true
}
}
    }
    /// return a pair of link and title
    fn parse_html_frac(&self, html: &str, selector: &str) -> Result<Vec<(Option<String>, String)>> {
        let (document, selector) = selector_parse_frac(html, selector)?;
        let mut pairs = vec![];
        // assume there is only one ele
        for e in document.select(&selector) {
            let link = e.value().attr("href");
            let title = e.text();
            pairs.push((
                link.map(|e| e.to_string()),
                title
                    .collect::<Vec<_>>()
                    .last()
                    .as_ref()
                    .unwrap()
                    .to_string(),
            ))
        }
        Ok(pairs)
    }
    /// enter chapter page ，parse html ul[id="downloads"] -> parse frac a ,get 2 eles,1st audio,2nd pdf
    fn parse_audio_page(
        &mut self,
        html: &str,
        selector: &str,
        sub_selector: &str,
    ) -> Result<AudioLink> {
        let htmls = parse_html_doc(&html, selector)?;
        let html = htmls.last();
        println!("parse_html_doc result {}", &html.as_ref().unwrap());
        let pairs = self.parse_html_frac(&html.as_ref().unwrap(), sub_selector)?;
        let audio_link = pairs.first();
        let pdf = pairs.get(1);
        if audio_link.is_none() || pdf.is_none() {
            return Err(ApplicationError::ValueNotFound(
                "audio_link or pdf link item not found".into(),
            ));
        }
        let (audio_link, _) = pairs.first().as_ref().unwrap();
        let (pdf, _) = pairs.get(1).as_ref().unwrap();
        println!("link of audio and pdf {:?}, {:?}", audio_link, pdf);
        let audio = AudioLink::new(
            Some(audio_link.as_ref().unwrap().to_string()),
            pdf.as_ref().unwrap().to_string(),
        );

        Ok(audio)
    }
    /// palin text(text is arranged in a vec of tag p) : parse tag p ,get text ,then join text (note \n,add if not)
    fn parse_plain_text(&self, document: &Html, selector: &Selector) -> Result<String> {
        let ele = document.select(&selector).next();
        let text = if let Some(e) = ele {
            let (doc, sel) = selector_parse_frac(&e.html(), "p")?;
            let txt = doc
                .select(&sel)
                .into_iter()
                .map(|e| e.text())
                .collect::<Vec<_>>();
            let t = txt
                .iter()
                .map(|e| e.to_owned().collect::<Vec<_>>().join(""))
                .collect::<Vec<_>>()
                .join("\n");
            t
        } else {
            return Err(ApplicationError::ParseHtmlSelector(
                "parse_plain_text error".into(),
            ));
        };
        Ok(text)
    }
    /// parse audio  , audio:tag "source" ,first matched ele,get attr "src".
    fn parse_audi_link(&self, document: &Html, selector: &Selector) -> Result<String> {
        let ele = document.select(&selector).next();
        if let Some(e) = ele {
            let (doc, sel) = selector_parse_frac(&e.html(), "source")?;
            let el = doc.select(&sel).next();
            let ele = el.as_ref().unwrap();
            let src = match ele.value().attr("src") {
                Some(s) => Ok(s.to_string()),
                None => Err(ApplicationError::ValueNotFound("get attr src none".into())),
            };
            Ok(src?)
        } else {
            return Err(ApplicationError::ValueNotFound("get selector none".into()));
        }
    }
    async fn update_audio(&mut self) -> Result<()> {
        println!("update audio of chapter {}", self.chapter_name);
        let html = self.request_text(&self.capter_link).await?;
        let (document, selector) =
            selector_parse_frac(&html, r#"div[id="i_apologize_for_the_soup"]"#)?;
        let audio_link = self.parse_audi_link(&document, &selector)?;
        let text = self.parse_plain_text(&document, &selector)?;
        let audio = AudioLink::new(Some(audio_link), text);
        self.set_audio(Some(audio));
        Ok(())
    }
    fn set_audio(&mut self, audio: Option<AudioLink>) {
        self.audio = audio;
    }

    fn chapter_name(&self) -> String {
        self.chapter_name.to_string()
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub(crate) struct AudioLink {
    audio_link: Option<String>,
    /// contain mulit-line plain text string
    text: String,
}

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

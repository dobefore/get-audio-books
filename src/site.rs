use crate::error::{ApplicationError, Result};
use async_trait::async_trait;
use json;
use regex::Regex;
use scraper::{Html, Selector};
use tokio::io::AsyncWriteExt;
use std::{borrow::Borrow, path::Path};
///https://freeaudiobooksonline.net/audiobook-list/recommended-audiobooks
#[derive(Default, Debug)]
pub(crate) struct PagePattern1 {
    root_link: String,
    bookpages1: Option<Vec<BookPage1>>,
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
        Ok(text) }
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
            return Err(ApplicationError::ParseHtmlSelector("选择的元素为空或不存在".into()));
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
            Err(_) =>return Err(ApplicationError::ParseHtmlSelector(format!(
                "parse {} element error",
                selector
            ))),
        };
        let mut bgs = vec![];
        if document.select(&selector.clone()).collect::<Vec<_>>().is_empty() {
            return Err(ApplicationError::ParseHtmlSelector("选择的元素为空或不存在".into()));
        }
        for element in document.select(&selector) {
            // println!("{:?}",element.value().attr("href"))
            let bg = self.parse_html_frac(&element.html(), "a")?;
            bgs.push(bg);
            // println!("{:?}",element.value())
        }
        Ok(bgs)
    }
    async fn down_local(&self, book_title: &str, parts: &[Part],real_book_title:&str) -> Result<()> {
        // create dir with book_title as its name ,write each part to this folder
        // skip if dir exists
        let p = Path::new("output");
        if !p.exists() {
            tokio::fs::create_dir(p).await?;
        }
        let error_log=p.join("error.log");
        let book_dir = p.join(real_book_title);
        if !book_dir.exists() {
            tokio::fs::create_dir(&book_dir).await?;
        }
println!("downloading book {}",book_title);
        // loop parts
        for p in parts {
            // get link and title (as filename)
            let link = &p.audio_link;
            let fname = format!("{}.mp3", p.audio_title);
            if link.is_none() {
               let mut f= tokio::fs::OpenOptions::new().create(true).append(true).open(&error_log).await?;
           f.write(format!("book part link is missing,whhose book name,part title are {},{}",book_title,p.audio_title).as_bytes()).await?;
        continue;  
        }
            println!("downloading book part {} from link {}",p.audio_title,link.as_ref().unwrap());
if book_dir.join(&fname).exists() {
    continue;
}
            let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
           
            let resp = reqwest::ClientBuilder::new()
                .user_agent(pc)
                .build()?
                .get(link.as_ref().unwrap())
                .send()
                .await?
                .bytes()
                .await?;
            tokio::fs::write(book_dir.join(fname), resp).await?;
        }

        Ok(())
    }
    pub(crate) async fn down(&mut self) -> Result<()> {
        let text = self.request_text(&self.root_link).await?;
        let bgs = self.parse_html_doc(
            &text,
            r#"p[class="name product-title woocommerce-loop-product__title"]"#,
        )?;
        self.update_bookpages(bgs)?;
        self.update_parts().await?;
        // loop BookPage1 to down
        if let Some(bgs) = self.bookpages1.as_ref() {
            let mut n=0;
            for bg in bgs {
                self.down_local(bg.book_title.as_ref().unwrap(), bg.parts.as_ref().unwrap(),&n.to_string())
                    .await?;
                    n+=1;
            }
        }
        // test audio link valid.
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
        let   link=  if link.is_empty() {
               Err( ApplicationError::ValueNotFound("empty value in link".into()))

            }else {
                Ok(link)
            };
          let link=  match link {
                Ok(l)=>Some(l),
                Err(_)=>None
            };
            let part = Part::new(link.map(|w|w.to_string()), title.to_string());
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
            return Err(ApplicationError::ParseHtmlSelector("选择的元素为空或不存在".into()));
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
        println!("updating parts from book link {:?}", self.book_link.as_ref());

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
struct Part {
    audio_link: Option<String>,
    audio_title: String,
}

impl Part {
    fn new(audio_link: Option<String>, audio_title: String) -> Self {
        Self {
            audio_link,
            audio_title,
        }
    }
}
#[derive(Debug, Default)]
pub(crate) struct Lit2Go {
    pub(crate) root_site: String,
    pub(crate) book_site: String,
    bookpages: Option<Vec<BookPage>>,
}

impl Lit2Go {
    async fn down(&self) -> Result<()> {
        Ok(())
    }
    fn parse_audio_link_for_each_chapter(&mut self) -> Result<()> {
        Ok(())
    }
    fn parse_chapters_for_each_book(&mut self) -> Result<()> {
        Ok(())
    }
    fn parse_root_html(&mut self, html: &str) -> Result<()> {
        let document = Html::parse_document(html);
        let selector = match Selector::parse("li") {
            Ok(s) => Ok(s),
            Err(_) => Err(ApplicationError::ParseHtmlSelector(
                "parse li element error".into(),
            )),
        };

        for element in document.select(&selector?) {
            assert_eq!("li", element.value().name());
        }
        Ok(())
    }
}
/// convert a specific string to an instance of site struct.e.g. "Lit2Go" -> [`Lit2Go`]
pub(crate) fn convert_to_site(s: &str) -> Option<Sites> {
    match s {
        "Lit2Go" => Some(Sites::Lit2Go(Lit2Go::new())),
        _ => None,
    }
}
#[async_trait]
impl Down for Lit2Go {
    fn parse_html(&self, html: &str) -> Result<()> {
        let document = Html::parse_document(html);
        let selector = match Selector::parse("li") {
            Ok(s) => Ok(s),
            Err(_) => Err(ApplicationError::ParseHtmlSelector(
                "parse li element error".into(),
            )),
        };

        for element in document.select(&selector?) {
            assert_eq!("li", element.value().name());
        }
        Ok(())
    }

    async fn request_link(&self, link: &str) -> Result<String> {
        Ok(reqwest::get(link).await?.text().await?)
    }
    async fn download_all(&mut self) -> Result<()> {
        // request book info site and get html of book page
        let book_info = self.request_link(&self.book_site).await?;
        // parse html str to get a vec of page links of all books,
        self.parse_root_html(&book_info)?;
        // iterate book page links, request each book link and get html of each book page
        // parse html of each book page to get a vec of  chapter links and titles
        self.parse_chapters_for_each_book()?;
        // request each chapter link to get its html stream and parse it to get audio link(.mp3) and
        // text link(pdf)
        self.parse_audio_link_for_each_chapter()?;
        // download audio and text to local fs. path output/bookname/chaptername/
        self.down().await?;
        Ok(())
    }
}
#[async_trait]
trait Down {
    /// download_all books
    async fn download_all(&mut self) -> Result<()>;
    ///  request each book link and return response text in String
    async fn request_link(&self, link: &str) -> Result<String>;
    /// parse html document and return selected elements
    fn parse_html(&self, html: &str) -> Result<()>;
}

impl Lit2Go {
    fn new() -> Self {
        Self {
            root_site: "https://etc.usf.edu/lit2go/".into(),
            book_site: "https://etc.usf.edu/lit2go/books/".into(),
            ..Self::default()
        }
    }

    /// return books by alphabetic index.
    fn index_by_alphabet() {}
}
pub(crate) enum Sites {
    Lit2Go(Lit2Go),
}
impl Sites {
    /// download_all books from a specific website
    pub(crate) async fn download_all(&mut self) -> Result<()> {
        match self {
            Self::Lit2Go(lg) => {
                lg.download_all().await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
struct BookPage {
    book_name: String,
    book_link: String,
    chapters: Vec<Chapter>,
}

impl BookPage {
    fn new() -> Self {
        Self::default()
    }
}
#[derive(Debug, Default)]
struct Chapter {
    chapter_name: String,
    capter_link: String,
    audiolinks: Option<Vec<AudioLink>>,
}

impl Chapter {
    fn parse_link(&mut self) {}
}

#[derive(Debug, Default)]
struct AudioLink {
    audio_link: String,
    pdf_link: String,
}

use crate::{
    download,
    error::{ApplicationError, Result},
    fileops::{
       open_as_append, open_as_append_async,
         open_as_read, open_as_write,
    },
    site::{strip_invalid_str, AudioLink, Down, Utils},
    utils::request_text,
};
use futures::future::join_all;
use indicatif:: ProgressBar;
use json;
use regex::{internal::Input, Regex};
use scraper::{element_ref::Select, ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    cell::{Cell, RefCell, RefMut},
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    io::SeekFrom,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    result,
    str::Bytes,
    sync::Arc,
    time::Duration,
};
use tokio::{io::AsyncWriteExt, sync::Mutex};

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
    book_count: Option<u8>,
    bookpages: Option<Vec<BookPage>>,
}
impl Utils for Lit2Go {
    type Item = Option<Vec<BookPage>>;
}
async fn audio_op(
    name: String,
    link: String,
    chapter_dir: PathBuf,
    error_file: PathBuf,
    book_name: String,
    text_file_dir: PathBuf,
) -> Result<()> {
    // in case some chapters have identical names,add book_name
    let chapter_path = chapter_dir.join(format!(
        "{}{}.txt",
        strip_invalid_str(&book_name),
        strip_invalid_str(&name)
    ));
    if chapter_path.exists() {
        return Ok(());
    }
    let text_path = text_file_dir.join(format!(
        "{}{}.txt",
        strip_invalid_str(&book_name),
        strip_invalid_str(&name)
    ));
    if text_path.exists() {
        return Ok(());
    }
    // c.update_audio(text_file_path, book_name, error_log)
    let html = request_text(&link).await?;
    let error_file = Arc::new(std::sync::Mutex::new(open_as_append(&error_file)?));
    let mut c = Chapter::new(name, link);
    c.update_audio_async(&text_path, &book_name, error_file, &c.chapter_name(), html)
        .await?;
    let mut f = open_as_write(&chapter_path)?;
    // let writer = std::io::BufWriter::new(f);
    let c_str = serde_json::to_string(&c)?;
    f.write_all(&c_str.as_bytes())?;
    // serde_json::to_writer(writer, &c_str)?;
    Ok(())
}
impl Lit2Go {
    fn count_actual_books(&self) -> u8 {
        if let Some(bgs) = self.bookpages.as_ref() {
            bgs.len() as u8
        } else {
            0
        }
    }
    /// remove from book page by book name
    ///
    /// The Story of Siegfried

    fn equal_non_zero(&self) -> bool {
        let acctual = self.count_actual_books();
        if acctual != 0 {
            if acctual.eq(&self.book_count.unwrap()) {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    // /// check whether chapters in each book are fully parsed by check count equal
    // fn check_audio_sanity_of_chapters(&self) -> bool {
    //     self.bookpages.as_ref().unwrap().iter().all(|e| {
    //         e.chapters
    //             .as_ref()
    //             .unwrap()
    //             .iter()
    //             .all(|c| c.audio_sanity())
    //     })
    // }
    /// check whether chapters in each book are fully parsed by check count equal
    fn check_chapter_sanity_of_books(&self) -> bool {
        self.bookpages
            .as_ref()
            .unwrap()
            .iter()
            .all(|e| e.equal_non_zero())
    }
    // /// chapter_name as the filename of audio and odf
    // async fn down_local(
    //     &self,
    //     book_title_code: &str,
    //     book_title: &str,
    //     chapter_name: &str,
    //     audio_link: &str,
    //     pdf_link: &str,
    //     output: &Path,
    // ) -> Result<()> {
    //     // create dir with book_title as its name ,write each part to this folder
    //     // skip if dir exists
    //     if !output.exists() {
    //         tokio::fs::create_dir(&output).await?;
    //     }
    //     let book_dir = output.join(book_title_code);
    //     if !book_dir.exists() {
    //         tokio::fs::create_dir(&book_dir).await?;
    //     }
    //     println!("downloading book {}", book_title);

    //     let audio_fname = format!("{}.mp3", chapter_name);
    //     let pdf_fname = format!("{}.pdf", chapter_name);

    //     println!(
    //         "downloading book chapter {} from link {}",
    //         chapter_name, audio_link
    //     );

    //     // tokio::process::Command::new("curl").args(&[part_link.as_ref().unwrap(),"-o",book_dir.join(fname).to_str().unwrap(),"-s"]).spawn().expect("");
    //     // std::process::Command::new("curl").args(&[part_link.as_ref().unwrap(),"-o",book_dir.join(fname).to_str().unwrap(),"-s"]).spawn().expect("");
    //     let audio_bytes = self.request_bytes(&audio_link).await?;
    //     // let pdf_bytes = self.request_bytes(&pdf_link).await?;

    //     tokio::fs::write(book_dir.join(audio_fname), audio_bytes).await?;
    //     // tokio::fs::write(book_dir.join(pdf_fname), pdf_bytes).await?;
    //     Ok(())
    // }

    /// download audio to local.
    ///
    /// the limit task is 15.
    ///
    /// links: (audio path to be written to,audio link)
    async fn download_audio(&self, links: Vec<(PathBuf, String)>) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let limit = 15;

        let chs = group_by_range(links, limit);
        tokio::spawn(async move {
            for i in chs {
                if let Err(_) = tx.send(i).await {
                    println!("receiver dropped");
                    return;
                }
            }
        });

        while let Some(a) = rx.recv().await {
            download_local(a).await?;
        }
        Ok(())
    }
    /// download audio to local.
    ///
    /// download audio text to file if pdf text is used,else copy plain text to corresponding
    /// chapter folder.
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub(crate) async fn down(&self, output: &Path) -> Result<()> {
        // read links from file via serde_json
        let lit2go_file = output.join("lit2go.txt");

        let f = open_as_read(&lit2go_file)?;
        let reader = std::io::BufReader::new(f);
        let lg: Lit2Go = serde_json::from_reader(reader)?;
        let mut links = vec![];
        let mut audio_text_links = vec![];

        for bg in lg.bookpages.as_ref().unwrap() {
            let bgna = strip_invalid_str(&bg.book_name());
            let book_dir = output.join(bgna);
            if !book_dir.exists() {
                tokio::fs::create_dir(&book_dir).await?;
            }
            for c in bg.chapters.as_ref().unwrap() {
                if !c.audio.is_none() {
                    let a = c.audio.as_ref().unwrap().audio_link();
                    let audio_text = c.audio.as_ref().unwrap().text();
                    if a.is_some() {
                        let l = a.as_ref().unwrap();
                        let chapter_name = strip_invalid_str(&c.chapter_name());
                        links.push((
                            book_dir.clone().join(format!("{}.mp3", chapter_name)),
                            l.to_string(),
                        ));
                        audio_text_links.push((
                            book_dir.clone().join(format!("{}.pdf", chapter_name)),
                            audio_text,
                        ));
                    }
                }
            }
        }

        self.download_audio(links).await?;
        self.download_audio_text(audio_text_links).await?;
        Ok(())
    }

    ///construct a channel to transport 15 tasks each time
    ///
    /// arguments include `(pdf file path format,audio text link)`respectively,pdf path is allowed
    /// if audio uses pdf text. audio text link is either pdf link or plain text local file path
    /// # Errors
    ///
    /// This function will return an error if .
    async fn download_audio_text(&self, audio_text_links: Vec<(PathBuf, String)>) -> Result<()> {
        // split vec to a collection of vecs that each vec consists of 15 items
        let tasks = group_by_range(audio_text_links, 15);

        // construct basic channel and spawn model
        let (tx, mut rx) = tokio::sync::mpsc::channel(15);
        tokio::spawn(async move {
            for i in tasks {
                if let Err(_) = tx.send(i).await {
                    println!("receiver dropped");
                    return;
                }
            }
        });
        while let Some(i) = rx.recv().await {
            self.run_download_audio_text(i).await?;
        }

        Ok(())
    }
    /// run 15 tasks cocurrently each time.
    ///
    /// construct cocurrent environment for acutal task handler to run.
    async fn run_download_audio_text(&self, tasks: Vec<(PathBuf, String)>) -> Result<()> {
        // add progress bar
        let bar = ProgressBar::new(tasks.len() as u64);
        bar.println("batch tasks run ******************");
        let mut handles = vec![];
        for (pdf_path, text_link) in tasks {
            let bar = bar.clone();
            handles.push(tokio::spawn(async move {
                loop {
                    if let Err(e) =
                        Lit2Go::handle_one_audio_text((pdf_path.clone(), text_link.clone())).await
                    {
                        println!("{}", e);
                    } else {
                        bar.inc(1);
                        break;
                    }
                    bar.println("task fails,retry after duration of 3s");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }));
        }

        let _ = join_all(handles).await;
        bar.finish();
        Ok(())
    }
    /// every task is expected to download one audio pdf text from the Internet or copy local plain text file to
    /// corresponding chapter folder.
    async fn handle_one_audio_text((pdf_path, link): (PathBuf, String)) -> Result<()> {
        // parse link to determine whether it is a pdf web link or local file path
        let link_path = Path::new(&link);
        if link_path
            .extension()
            .as_ref()
            .unwrap()
            .to_str()
            .as_ref()
            .unwrap()
            .to_owned()
            == "txt"
        {
            // it's a file path,so copy file to chapter folder
            if let Some(p) = pdf_path.parent() {
                let to = p.join(
                    link_path
                        .file_name()
                        .as_ref()
                        .unwrap()
                        .to_str()
                        .as_ref()
                        .unwrap(),
                );
                tokio::fs::copy(link_path, to).await?;
            };
        } else {
            match download::down(4, link, pdf_path).await {
                Some(_) => return Ok(()),
                None => return Err(ApplicationError::Download("download pdf error".into())),
            }
        }
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
            if title.contains("The Story of Siegfried") {
                continue;
            }
            let bg = BookPage::new(title, link.as_ref().unwrap().to_string());
            bgs.push(bg);
        }
        Ok(bgs)
    }
    /// create folder for each book,write audio links to a file located in its corresponding book folder
    ///
    /// create chapter folder for each link according to their downloaded mp3 file name,
    ///
    ///
    /// This function will return an error if .
    pub(crate) async fn create_book_folders(&self) -> Result<()> {
        Ok(())
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
        if !output.exists() {
            tokio::fs::create_dir(&output).await?;
        }
        if lit2go_file.exists() {
            let f = open_as_read(&lit2go_file)?;
            let reader = std::io::BufReader::new(f);
            let lg: Lit2Go = serde_json::from_reader(reader)?;
            self.set_book_count(lg.book_count);
            self.set_bookpages(lg.bookpages().map(|e| e.to_owned()));
            if !self.equal_non_zero() {
                return Err(ApplicationError::ValueNotEqual(format!(
                    "book numbers are not equal, total{:?} actual {:?}",
                    self.book_count,
                    self.count_actual_books()
                )));
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
        if !self.check_chapter_sanity_of_books() {
            // if encountered error,skip this book and log it
            match self.update_chapters_of_each_book(&error_log).await {
                Ok(_) => {}
                Err(_) => {
                    let lg_str = serde_json::to_string(&self)?;
                    std::fs::write(&lit2go_file, lg_str)?;
                    return Err(ApplicationError::UpdateLit2go(
                        "update_chapters_of_each_book".into(),
                    ));
                }
            };
            let lg_str = serde_json::to_string(&self)?;
            std::fs::write(&lit2go_file, lg_str)?;
        }
        println!("update chapters done");

        let text_dir = output.join("text");
        if !text_dir.exists() {
            tokio::fs::create_dir(&text_dir).await?;
        }
        self.update_audio_of_each_chapter_async(&output).await?;
        // if !self.check_audio_sanity_of_chapters() {
        //     match self
        //         .update_audio_of_each_chapter(&text_dir, &error_log)
        //         .await
        //     {
        //         Ok(_) => {}
        //         Err(_) => {
        //             let lg_str = serde_json::to_string(&self)?;
        //             std::fs::write(&lit2go_file, lg_str)?;
        //             return Err(ApplicationError::UpdateLit2go(
        //                 "update_audio_of_each_chapter".into(),
        //             ));
        //         }
        //     };
        //     let lg_str = serde_json::to_string(&self)?;
        //     std::fs::write(&lit2go_file, lg_str)?;
        // }

        println!("set audio done");
        let lgl = open_as_write(&lit2g_links)?;
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
                        if chapter.audio.is_none() {
                            continue;
                        }
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

    ///Before request webpage,first check whether the serialized file exists or not according to
    /// chapter name. If not ,request html page of chapter link and store it into memory variable  .
    ///
    /// parse html stream to get audio link and audio text pdf link/plain text.
    ///
    /// create a new instance of [`Chapter`] and serialize it to a file once parsing work is done
    ///
    /// auguments it will receive. `chapter name`,`chapter link`,`book name`,`output path`,
    ///
    /// this fn will be run in join mode.that means running cocurrently.
    async fn get_audio(chapter: Vec<(String, String, String)>, output: &Path) -> Result<()> {
        let mut handles = vec![];
        let chapter_dir = output.join("chapters");
        if !chapter_dir.exists() {
            tokio::fs::create_dir(&chapter_dir).await?;
        }
        let error_file = output.join("error.txt");
        let text_file_dir = output.join("text");
        if !text_file_dir.exists() {
            tokio::fs::create_dir(&text_file_dir).await?;
        }
        let bar = ProgressBar::new(chapter.len() as u64);
        bar.println("batch tasks run ******************");
        for (name, link, book_name) in chapter {
            let name = name.clone();
            let chapter_dir = chapter_dir.clone();
            let error_file = error_file.clone();
            let text_file_dir = text_file_dir.clone();
            let bar = bar.clone();
            let mut retry_limit = 0;
            handles.push(tokio::spawn(async move {
                loop {
                    bar.println(format!("task {} of  {}running", name, book_name));
                    if let Err(e) = audio_op(
                        name.clone(),
                        link.clone(),
                        chapter_dir.clone(),
                        error_file.clone(),
                        book_name.clone(),
                        text_file_dir.clone(),
                    )
                    .await
                    {
                        println!("{}", e);
                    } else {
                        bar.inc(1);
                        break;
                    }
                    if retry_limit >= 3 {
                        break;
                    }
                    bar.println("task fails,retry after duration of 3s");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    retry_limit += 1;
                }
            }));
        }
        let ret = join_all(handles).await;
        bar.finish();
        Ok(())
    }
    /// the limit task is 15,use channel to impl it;.
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    async fn run_get_audio(chapter: Vec<(String, String, String)>, output: &Path) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(15);
        let limit = 15;
        //    set chapter dir and error dir
        let chs = group_by_range(chapter, limit);

        tokio::spawn(async move {
            for i in chs {
                if let Err(_) = tx.send(i).await {
                    println!("receiver dropped");
                    return;
                }
            }
        });

        while let Some(i) = rx.recv().await {
            Lit2Go::get_audio(i, &output.clone()).await?;
        }
        Ok(())
    }
    /// deserialize. file to [`Chapter`] by chapter name.
    async fn de_chapter(chapter_path: &Path) -> Result<Chapter> {
        let f = open_as_read(&chapter_path)?;
        let reader = std::io::BufReader::new(f);
        let ch: Chapter = serde_json::from_reader(reader)?;
        Ok(ch)
    }
    /// push chapter links to a vec.download html files whose filenames are speicfied by chapter name
    /// to a folder .
    /// # how to determine who the audio links belong to ?
    /// 1. BookPage (used to deserilize according to chapter name)
    /// loop books,loop chapters,first derserilize chapter file Chapter. after chapters done,set chapters
    /// and at last set books.
    ///
    ///2. Chapter (serialize to file whose filename is specified by chapter name
    ///  after each download task is done and audio links are parsed) (destinguish html file and
    /// serilized file,maybe in different folders?)
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    async fn update_audio_of_each_chapter_async(&mut self, output: &Path) -> Result<()> {
        // dir to store seriailzed chapter files
        let chapter_dir = output.join("chapters");
        // loop bookpages and thus chapters, make them a vec of (chapter name,chapter linka).
        let mut cs = vec![];
        let bgs = self.bookpages();
        for b in bgs.unwrap() {
            for c in b.chapters().unwrap() {
                cs.push((c.chapter_name, c.capter_link, b.book_name()));
            }
        }
        // limit tasks is 15;
        Lit2Go::run_get_audio(cs, &output.clone()).await?;
        // deserialixe: create a new instance of lit2go,set bookpages and chapters,and then loop it recursively
        // to chapter,deserialize file whose file name is chapter name to Chapter,set Chapter.
        let mut bgss = vec![];
        for b in bgs.unwrap() {
            let mut bg = BookPage::new(b.book_name(), b.book_link());
            let chs = b.chapters();
            let mut chapters = vec![];

            for cc in chs.as_ref().unwrap() {
                let c = Chapter::new(cc.chapter_name(), cc.capter_link());
                // deserialize chapter
                let chapter_path = chapter_dir.join(format!(
                    "{}{}.txt",
                    strip_invalid_str(&b.book_name()),
                    strip_invalid_str(&c.chapter_name())
                ));
                let ch = match Lit2Go::de_chapter(&chapter_path).await {
                    Ok(c) => c,
                    Err(_) => {
                        return Err(ApplicationError::UpdateLit2go(format!(
                            "deserialize error {}",
                            chapter_path.display()
                        )))
                    }
                };
                chapters.push(ch);
            }
            bg.set_chapters(Some(chapters));
            bgss.push(bg);
        }
        self.set_bookpages(Some(bgss));

        // set bookpages
        Ok(())
    }
    // async fn update_audio_of_each_chapter(
    //     &mut self,
    //     text_file_path: &Path,
    //     error_file: &Path,
    // ) -> Result<()> {
    //     // loop ops
    //     let mut f = open_as_append_async(error_file).await?;
    //     let mut book_num = 0;
    //     for bg in self.bookpages.as_mut().unwrap() {
    //         match bg
    //             .update_audio(text_file_path, &bg.book_name(), error_file, book_num)
    //             .await
    //         {
    //             Ok(_) => {
    //                 book_num += 1;
    //             }
    //             Err(_) => {
    //                 append_str_async(&mut f, &format!("{}", bg.book_name)).await?;
    //                 return Err(ApplicationError::UpdateLit2go(
    //                     "update_audio_of_each_chapter".into(),
    //                 ));
    //             }
    //         };
    //     }
    //     Ok(())
    // }
    async fn update_chapters_of_each_book(&mut self, error_file: &Path) -> Result<()> {
        // loop ops
        // let mut f = open_as_append_async(error_file).await?;
        for bg in self.bookpages.as_mut().unwrap() {
            bg.update_chapters().await?;
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
async fn download_local(v: Vec<(PathBuf, String)>) -> Result<()> {
    let bar = ProgressBar::new(v.len() as u64);
    bar.println("batch tasks run ******************");
    let mut handles = vec![];
    for (p, l) in v {
        let bar = bar.clone();
        handles.push(tokio::spawn(async move {
            loop {
                bar.println(format!("downloading {}", p.clone().display()));
                if let Some(()) = download::down(10, l.clone(), p.clone()).await {
                    bar.inc(1);
                    break;
                }
                bar.println(format!("任务 {} 失败! 3s后重试!", p.display()));
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }));
    }
    join_all(handles).await;
    bar.finish();

    Ok(())
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

impl Down for Lit2Go {}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub(crate) struct BookPage {
    book_name: String,
    book_link: String,
    /// show how many chapters this book will have
    chapter_count: Option<u8>,
    chapters: Option<Vec<Chapter>>,
}
impl Utils for BookPage {
    type Item = Option<Vec<Chapter>>;
}

impl Down for BookPage {}
/// get audio link and text link
///
/// return [`Chapter`]s
//
// c:Mutex< Chapter>,
// async fn get_audio( book_name: String, error_log: PathBuf, text_file_path: PathBuf) {
//    let mut c=Chapter::new("chapter_name".to_string(), "capter_link".to_string());
//     c.update_audio(&text_file_path, &book_name, &error_log)
//         .await
//         .unwrap();
//     // return Ok(());
//     //
// }
// async fn update_audio(
//     mut c: Chapter,
//     text_file_path: &Path,
//     book_name: &str,
//     error_log: &Path,
// ) -> Result<()> {
//     if c.audio.is_some() {
//         println!(" audio seems existing in chapter {}", c.chapter_name);
//         return Ok(());
//     }
//     println!("update audio of chapter {}", c.chapter_name);
//     let html = match c.request_text(&c.capter_link).await {
//         Ok(h) => {
//             if h.trim().is_empty() {
//                 Err(ApplicationError::ValueNotFound(
//                     "request text empty,maybe network error,retry".into(),
//                 ))
//             } else {
//                 Ok(h)
//             }
//         }
//         Err(_) => Err(ApplicationError::ValueNotFound(
//             "maybe network error,retry".into(),
//         )),
//     };
//     let html = html?;
//     let (document, selector) = selector_parse_frac(&html, r#"div[id="i_apologize_for_the_soup"]"#)?;
//     if let Ok(audio_link) = c.parse_audi_link(&document, &selector) {
//         // ignore the situation where the link is an empty str
//         let audio_text = c
//             .parse_pdf_or_text(
//                 &html,
//                 &document,
//                 &selector,
//                 r#"ul[id="downloads"]"#,
//                 "a",
//                 text_file_path,
//             )
//             .await?;
//         let audio = AudioLink::new(Some(audio_link), audio_text);
//         c.set_audio(Some(audio));
//     } else {
//         // allow audio to be absent,but skip and log it.
//         let mut f = open_as_append_async(error_log).await?;
//         append_str_async(
//             &mut f,
//             &format!("chapter name: {} from book: {}", c.chapter_name, book_name),
//         )
//         .await?;
//     }

//     Ok(())
// }
fn parse_plain_text(document: &Html, selector: &Selector) -> Result<String> {
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
fn parse_pdf_or_text(
    html: &str,
    text_document: &Html,
    text_selector: &Selector,
    pdf_selector: &str,
    pdf_subselector: &str,
    text_path: &Path,
    (book_name, chapter_name): (&str, &str),
) -> Result<String> {
    let (pdf_html, pdf_sel) = selector_parse_doc(html, pdf_selector)?;
    let s = if let Some(e) = pdf_html.select(&pdf_sel).next() {
        let (pdf_html, pdf_sel) = selector_parse_frac(&e.html(), &pdf_subselector)?;
        let el = pdf_html.select(&pdf_sel).collect::<Vec<_>>();
        if !el.is_empty() {
            // get element according to different vec len
            let len = el.len();
            let pdf = if len == 1 {
                // this situation is true only when audio link is present
                // println!("using plain text for audio text");
                let text = parse_plain_text(&text_document, &text_selector)?;
                std::fs::write(&text_path, text)?;
                let p = format!("{}", text_path.display());
                // println!("get plain text path {}", p);
                p
            } else {
                // items len include 2,3,4
                let e = el.get(1);
                let pdf = e
                    .unwrap()
                    .value()
                    .attr("href")
                    .as_ref()
                    .unwrap()
                    .to_string();
                // println!("using pdf link for audio text");
                // println!("get pdf link : {}", &pdf);
                pdf
                // // return error
                // return Err(ApplicationError::ParseHtmlSelector(
                //    format!( "pdf ele len two large: {},{}",book_name,chapter_name)
                // ));
            };

            pdf
        } else {
            return Err(ApplicationError::ParseHtmlSelector(
                "parse pdf tag not found".into(),
            ));
        }
    } else {
        // println!("using plain text for audio text");
        let text = parse_plain_text(&text_document, &text_selector)?;

        std::fs::write(&text_path, text)?;
        format!("{}", text_path.display())
    };

    Ok(s)
}
fn parse_audi_link(document: &Html, selector: &Selector) -> Result<String> {
    let ele = document.select(&selector).next();
    if let Some(e) = ele {
        let (doc, sel) = selector_parse_frac(&e.html(), "source")?;
        let el = doc.select(&sel).next();
        let ele = match el.as_ref() {
            Some(e) => e,
            None => {
                return Err(ApplicationError::ParseHtmlSelector(
                    "srouce tag not found".into(),
                ))
            }
        };
        let src = match ele.value().attr("src") {
            Some(s) => Ok(s.to_string()),
            None => Err(ApplicationError::ValueNotFound("get attr src none".into())),
        };
        Ok(src?)
    } else {
        return Err(ApplicationError::ValueNotFound("get selector none".into()));
    }
}

impl BookPage {
    fn new(book_name: String, book_link: String) -> Self {
        Self {
            book_name,
            book_link,
            ..Default::default()
        }
    }

    fn count_actual_books(&self) -> u8 {
        if let Some(bgs) = self.chapters.as_ref() {
            bgs.len() as u8
        } else {
            0
        }
    }
    fn equal_non_zero(&self) -> bool {
        let acctual = self.count_actual_books();
        if acctual != 0 {
            if acctual == self.chapter_count.unwrap() {
                true
            } else {
                false
            }
        } else {
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
            }
            Err(_) => {
                let mut f = open_as_append_async("error.log".as_ref()).await?;
                f.write(self.book_name.as_bytes()).await?;
                return Err(ApplicationError::UpdateLit2go("update_chapters".into()));
            }
        };
        Ok(())
    }

    // /// maybe should return [`Chapter`]s,and set chapters.
    // async fn update_audio(
    //     &mut self,
    //     text_file_path: &Path,
    //     book_name: &str,
    //     error_log: &Path,
    //     book_num: u16,
    // ) -> Result<()> {
    //     if let Some(cs) = self.chapters.as_mut() {
    //         let mut chapter_num = 0;
    //         // push all chapters to handler,and join them to run cocurrently.

    //         for c in cs {
    //             // skip if audio link exists
    //             chapter_num += 1;
    //             if c.audio_sanity() {
    //                 println!("audio seems existing in chapter {}", c.chapter_name);
    //                 continue;
    //             }
    //             let text_file_path = text_file_path.join(format!("{}{}", book_num, chapter_num));
    //             c.update_audio(&text_file_path, book_name, error_log)
    //                 .await?;
    //         }
    //     }

    //     Ok(())
    // }
    fn book_name(&self) -> String {
        self.book_name.to_string()
    }

    pub(crate) fn set_chapters(&mut self, chapters: Option<Vec<Chapter>>) {
        self.chapters = chapters;
    }
    pub(crate) fn last_chapter(&self) -> &Chapter {
        self.chapters.as_ref().unwrap().last().as_ref().unwrap()
    }

    pub(crate) fn set_chapter_count(&mut self, chapter_count: Option<u8>) {
        self.chapter_count = chapter_count;
    }

    pub(crate) fn chapters(&self) -> Option<Vec<Chapter>> {
        self.chapters.to_owned()
    }

    pub(crate) fn book_link(&self) -> String {
        self.book_link.to_string()
    }
}
unsafe impl Sync for Chapter {}
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
    // /// return false if [`AudioLink`] is none
    // fn audio_sanity(&self) -> bool {
    //     if self.audio.is_some() {
    //         true
    //     } else {
    //         false
    //     }
    // }
    // /// return a pair of link and title
    // fn parse_html_frac(&self, html: &str, selector: &str) -> Result<Vec<(Option<String>, String)>> {
    //     let (document, selector) = selector_parse_frac(html, selector)?;
    //     let mut pairs = vec![];
    //     // assume there is only one ele
    //     for e in document.select(&selector) {
    //         let link = e.value().attr("href");
    //         let title = e.text();
    //         pairs.push((
    //             link.map(|e| e.to_string()),
    //             title
    //                 .collect::<Vec<_>>()
    //                 .last()
    //                 .as_ref()
    //                 .unwrap()
    //                 .to_string(),
    //         ))
    //     }
    //     Ok(pairs)
    // }
    // /// enter chapter page ，parse html ul[id="downloads"] -> parse frac a ,get 2 eles,1st audio,2nd pdf
    // fn parse_audio_page(
    //     &mut self,
    //     html: &str,
    //     selector: &str,
    //     sub_selector: &str,
    // ) -> Result<AudioLink> {
    //     let htmls = parse_html_doc(&html, selector)?;
    //     let html = htmls.last();
    //     println!("parse_html_doc result {}", &html.as_ref().unwrap());
    //     let pairs = self.parse_html_frac(&html.as_ref().unwrap(), sub_selector)?;
    //     let audio_link = pairs.first();
    //     let pdf = pairs.get(1);
    //     if audio_link.is_none() || pdf.is_none() {
    //         return Err(ApplicationError::ValueNotFound(
    //             "audio_link or pdf link item not found".into(),
    //         ));
    //     }
    //     let (audio_link, _) = pairs.first().as_ref().unwrap();
    //     let (pdf, _) = pairs.get(1).as_ref().unwrap();
    //     println!("link of audio and pdf {:?}, {:?}", audio_link, pdf);
    //     let audio = AudioLink::new(
    //         Some(audio_link.as_ref().unwrap().to_string()),
    //         pdf.as_ref().unwrap().to_string(),
    //     );

    //     Ok(audio)
    // }
    // /// palin text(text is arranged in a vec of tag p) : parse tag p ,get text ,then join text (note \n,add if not)
    // fn parse_plain_text(&self, document: &Html, selector: &Selector) -> Result<String> {
    //     let ele = document.select(&selector).next();
    //     let text = if let Some(e) = ele {
    //         let (doc, sel) = selector_parse_frac(&e.html(), "p")?;
    //         let txt = doc
    //             .select(&sel)
    //             .into_iter()
    //             .map(|e| e.text())
    //             .collect::<Vec<_>>();
    //         let t = txt
    //             .iter()
    //             .map(|e| e.to_owned().collect::<Vec<_>>().join(""))
    //             .collect::<Vec<_>>()
    //             .join("\n");
    //         t
    //     } else {
    //         return Err(ApplicationError::ParseHtmlSelector(
    //             "parse_plain_text error".into(),
    //         ));
    //     };
    //     Ok(text)
    // }
    // /// parse audio  , audio:tag "source" ,first matched ele,get attr "src".
    // ///
    // /// Part 2, Chapter 28 of book the age of innocense audio missing ,to fix this ,rmove that
    // /// item from that book,and decrease chapter count in lit2go.txt
    // fn parse_audi_link(&self, document: &Html, selector: &Selector) -> Result<String> {
    //     let ele = document.select(&selector).next();
    //     if let Some(e) = ele {
    //         let (doc, sel) = selector_parse_frac(&e.html(), "source")?;
    //         let el = doc.select(&sel).next();
    //         let ele = match el.as_ref() {
    //             Some(e) => e,
    //             None => {
    //                 return Err(ApplicationError::ParseHtmlSelector(
    //                     "srouce tag not found".into(),
    //                 ))
    //             }
    //         };
    //         let src = match ele.value().attr("src") {
    //             Some(s) => Ok(s.to_string()),
    //             None => Err(ApplicationError::ValueNotFound("get attr src none".into())),
    //         };
    //         Ok(src?)
    //     } else {
    //         return Err(ApplicationError::ValueNotFound("get selector none".into()));
    //     }
    // }
    // async fn parse_pdf_or_text(
    //     &self,
    //     html: &str,
    //     text_document: &Html,
    //     text_selector: &Selector,
    //     pdf_selector: &str,
    //     pdf_subselector: &str,
    //     text_dir: &Path,
    // ) -> Result<String> {
    //     let (pdf_html, pdf_sel) = selector_parse_doc(html, pdf_selector)?;
    //     let s = if let Some(e) = pdf_html.select(&pdf_sel).next() {
    //         println!("using pdf link for audio text");
    //         let (pdf_html, pdf_sel) = selector_parse_frac(&e.html(), &pdf_subselector)?;
    //         let el = pdf_html.select(&pdf_sel).collect::<Vec<_>>();
    //         if !el.is_empty() {
    //             // get element according to different vec len
    //             let len = el.len();
    //             let pdf = if len == 2 || len == 3 {
    //                 let e = el.get(1);
    //                 e.unwrap()
    //                     .value()
    //                     .attr("href")
    //                     .as_ref()
    //                     .unwrap()
    //                     .to_string()
    //             } else if len == 1 {
    //                 // maybe only pdf link is present
    //                 let e = el.first();
    //                 e.unwrap()
    //                     .value()
    //                     .attr("href")
    //                     .as_ref()
    //                     .unwrap()
    //                     .to_string()
    //             } else {
    //                 // return error
    //                 return Err(ApplicationError::ParseHtmlSelector(
    //                     "parse pdf tag two many ele len".into(),
    //                 ));
    //             };
    //             println!("get pdf link : {}", &pdf);
    //             pdf
    //         } else {
    //             return Err(ApplicationError::ParseHtmlSelector(
    //                 "parse pdf tag not found".into(),
    //             ));
    //         }
    //     } else {
    //         println!("using plain text for audio text");
    //         let text = self.parse_plain_text(&text_document, &text_selector)?;
    //         tokio::fs::write(&text_dir, text).await?;
    //         format!("{}", text_dir.display())
    //     };

    //     Ok(s)
    // }
    async fn update_audio_async(
        &mut self,
        text_file_path: &Path,
        book_name: &str,
        error_file: Arc<std::sync::Mutex<File>>,
        chapter_name: &str,
        html: String,
    ) -> Result<()> {
        let html = html;
        let (document, selector) =
            selector_parse_frac(&html, r#"div[id="i_apologize_for_the_soup"]"#)?;

        if let Ok(audio_link) = parse_audi_link(&document, &selector) {
            // ignore the situation where the link is an empty str
            let audio_text = parse_pdf_or_text(
                &html,
                &document,
                &selector,
                r#"ul[id="downloads"]"#,
                "a",
                text_file_path,
                (book_name, chapter_name),
            )?;
            let audio = AudioLink::new(Some(audio_link), audio_text);
            self.set_audio(Some(audio));
        } else {
            // let mut f=open_as_append(&error_file)?;
            error_file.lock().expect("lock error").write(
                format!("chapter name: {} from book: {}\n", chapter_name, book_name).as_bytes(),
            )?;
            // append_str(&mut f, &format!("chapter name: {} from book: {}",chapter_name, book_name))?;
        }

        Ok(())
    }
    // async fn update_audio(
    //     &mut self,
    //     text_file_path: &Path,
    //     book_name: &str,
    //     error_log: &Path,
    // ) -> Result<()> {
    //     if self.audio.is_some() {
    //         println!(" audio seems existing in chapter {}", self.chapter_name);
    //         return Ok(());
    //     }
    //     println!("update audio of chapter {}", self.chapter_name);
    //     let html = match self.request_text(&self.capter_link).await {
    //         Ok(h) => {
    //             if h.trim().is_empty() {
    //                 Err(ApplicationError::ValueNotFound(
    //                     "request text empty,maybe network error,retry".into(),
    //                 ))
    //             } else {
    //                 Ok(h)
    //             }
    //         }
    //         Err(_) => Err(ApplicationError::ValueNotFound(
    //             "maybe network error,retry".into(),
    //         )),
    //     };
    //     let html = html?;
    //     let (document, selector) =
    //         selector_parse_frac(&html, r#"div[id="i_apologize_for_the_soup"]"#)?;
    //     if let Ok(audio_link) = self.parse_audi_link(&document, &selector) {
    //         // ignore the situation where the link is an empty str
    //         let audio_text = self
    //             .parse_pdf_or_text(
    //                 &html,
    //                 &document,
    //                 &selector,
    //                 r#"ul[id="downloads"]"#,
    //                 "a",
    //                 text_file_path,
    //             )
    //             .await?;
    //         let audio = AudioLink::new(Some(audio_link), audio_text);
    //         self.set_audio(Some(audio));
    //     } else {
    //         // allow audio to be absent,but skip and log it.
    //         let mut f = open_as_append_async(error_log).await?;
    //         append_str_async(
    //             &mut f,
    //             &format!(
    //                 "chapter name: {} from book: {}",
    //                 self.chapter_name, book_name
    //             ),
    //         )
    //         .await?;
    //     }

    //     Ok(())
    // }

    fn set_audio(&mut self, audio: Option<AudioLink>) {
        self.audio = audio;
    }

    fn chapter_name(&self) -> String {
        self.chapter_name.to_string()
    }

    pub(crate) fn capter_link(&self) -> String {
        self.capter_link.to_string()
    }
}
fn group_by_range<T>(mut v: Vec<T>, range: u8) -> Vec<Vec<T>> {
    let mut g = vec![];
    loop {
        if v.is_empty() {
            break;
        }
        if v.len() < range.into() && !v.is_empty() {
            g.push(v);
            break;
        }
        let cs = v.drain(0..range as usize).collect::<Vec<_>>();
        g.push(cs);
    }
    g
}
#[test]
fn test_group_by_range() {
    let r = group_by_range(vec![1, 2, 3, 4, 5], 5);
    let mut r1 = vec![];
    r1.push(vec![1, 2, 3, 4, 5]);

    let r2 = (0..32).collect::<Vec<_>>();
    let mut expect = Vec::new();
    expect.push([15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29].to_vec());
    expect.push([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14].to_vec());
    expect.push([30, 31].to_vec());

    assert_eq!(r, r1);
    assert_eq!(expect.len(), group_by_range(r2, 15).len());
}

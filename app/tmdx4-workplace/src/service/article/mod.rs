//! 記事管理システムの実装

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::LazyLock};

#[derive(Deserialize, Serialize)]
pub struct ArticlesConfig {
  pub article_rootpath: String,
  pub articles_path: String,
}
impl Default for ArticlesConfig {
  fn default() -> Self {
    Self {
      article_rootpath: "./article".into(),
      articles_path: "./entries".into(),
    }
  }
}

/// 記事のID
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ArticleID(u64);

/// 記事のIDのマスタ
#[derive(Serialize, Deserialize)]
pub struct ArticleIDMaster(u64);
impl ArticleIDMaster {
  pub fn issue(&mut self) -> Result<ArticleID, Box<dyn std::error::Error>> {
    let r = ArticleID(self.0);
    self.0 = self.0.wrapping_add(1);
    rmp_serde::encode::write(
      &mut std::fs::File::open(&crate::CONFIG.service.articles.article_rootpath)?,
      &self,
    )?;
    Ok(r)
  }
}
impl Default for ArticleIDMaster {
  fn default() -> Self {
    Self(0u64)
  }
}
static ARTICLE_ID: LazyLock<parking_lot::Mutex<ArticleIDMaster>> = LazyLock::new(|| {
  let config = &crate::CONFIG.service.articles;
  let article_rootpath = config.article_rootpath.clone() + "/article_id_master.bin";
  match std::fs::File::open(&article_rootpath) {
    Ok(fp) => parking_lot::Mutex::new(rmp_serde::from_read(std::io::BufReader::new(fp)).unwrap()),
    Err(e) => match e.kind() {
      std::io::ErrorKind::NotFound => {
        let default = ArticleIDMaster::default();
        let articles_entries_path = article_rootpath.clone() + config.articles_path.as_str();
        if std::fs::exists(articles_entries_path.as_str()).unwrap() {
          std::fs::create_dir_all(article_rootpath.clone() + config.articles_path.as_str())
            .unwrap();
        }
        rmp_serde::encode::write(
          &mut std::io::BufWriter::new(std::fs::File::create(&article_rootpath).unwrap()),
          &default,
        )
        .unwrap();
        parking_lot::Mutex::new(default)
      }
      _ => panic!("{e}"),
    },
  }
});

/// 記事
#[derive(Serialize, Deserialize)]
pub struct ArticleData {
  id: ArticleID,
  title: String,
  body: String,
}

/// 記事提供サービス
pub struct ArticleService {
  table: HashMap<ArticleID, usize>,
  articles: Vec<Option<ArticleData>>,
  remove_queue: VecDeque<usize>,
}
impl ArticleService {
  pub fn new() -> Self {
    Self {
      table: HashMap::new(),
      articles: Vec::new(),
      remove_queue: VecDeque::new(),
    }
  }
  pub fn post(
    &mut self,
    title: String,
    body: String,
  ) -> Result<&ArticleData, Box<dyn std::error::Error>> {
    let aid = ARTICLE_ID.lock().issue()?;
    let index = if let Some(index) = self.remove_queue.pop_front() {
      self.articles[index] = Some(ArticleData {
        id: aid,
        title,
        body,
      });
      index
    } else {
      let i = self.articles.len();
      self.articles.push(Some(ArticleData {
        id: aid,
        title,
        body,
      }));
      i
    };
    if self.table.insert(aid, index).is_some() {
      panic!("Article ID is INVALID!")
    }
    Ok(self.articles[index].as_ref().unwrap())
  }
  pub fn remove(&mut self, aid: &ArticleID) -> Option<ArticleData> {
    let index = self.table.remove(aid)?;
    let art = self.articles.remove(index)?;
    self.remove_queue.push_back(index);
    Some(art)
  }
  pub fn request(&self, aid: &ArticleID) -> Option<&ArticleData> {
    self.articles[*self.table.get(aid)?].as_ref()
  }
  pub fn iter(&self) -> impl Iterator<Item = &ArticleData> {
    self
      .table
      .iter()
      .filter_map(|(_aid, index)| self.articles[*index].as_ref())
  }
}

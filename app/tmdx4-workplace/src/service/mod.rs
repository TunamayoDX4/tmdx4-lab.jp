//! # 各種提供機能の実装

use std::{
  borrow::Cow,
  collections::{BTreeMap, VecDeque},
};

use chrono::{DateTime, Utc};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct ServiceConfig {
  pub articles: ArticlesConfig,
  pub assets: AssetConfig,
}

#[derive(Deserialize, Serialize)]
pub struct ArticlesConfig {
  pub article_rootpath: String,
}
impl Default for ArticlesConfig {
  fn default() -> Self {
    Self {
      article_rootpath: "./articles".into(),
    }
  }
}

/// 記事
#[derive(Clone)]
struct Article {
  /// タイトル
  title: String,
  /// 本文
  body: String,
  /// 投稿日付
  upload_time: DateTime<Utc>,
  /// 更新日付
  update_time: DateTime<Utc>,
}
struct ArticleService {
  /// エントリのメモリアドレスを格納するハッシュテーブル
  /// データの1番要素は格納先のアドレス、2番要素はエントリがLFUにあるか
  table: HashMap<String, (usize, bool)>,
  /// エントリ情報そのものを格納するメモリ
  memory: Vec<Option<Article>>,
  /// キャッシュエントリ
  cache_entry: VecDeque<usize>,
  /// Probation領域を1としたProtected領域のキャパシティ
  slru_ratio: u64,
  /// Probation領域のキャパシティ
  probation_cap: u64,
}
impl ArticleService {
  /// 記事の投稿
  pub fn post(&self, article: Article) {}

  /// 記事をタイトルから得る
  pub fn request_from_title(&mut self, title: &str) -> Option<&Article> {
    if let Some(entry) = self.table.get(title) {
      self.memory[entry.0].as_ref()
    } else {
      todo!()
    }
  }
}

#[derive(Deserialize, Serialize)]
pub struct AssetConfig {
  pub assets_rootpath: String,
}
impl Default for AssetConfig {
  fn default() -> Self {
    Self {
      assets_rootpath: "./assets".into(),
    }
  }
}

enum AssetType {
  CustomStyle,
  Image,
  Blob,
}

struct AssetService {}

//! # 各種提供機能の実装

use std::collections::{BTreeMap, VecDeque};

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
  articles: Vec<Article>,
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

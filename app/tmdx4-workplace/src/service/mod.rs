//! # 各種提供機能の実装

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct ServiceConfig {
  pub articles: article::ArticlesConfig,
  pub assets: AssetConfig,
}

pub mod article;

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

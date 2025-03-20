//! メインページの各種実装

use crate::{COMMON_CSS, MAIN_CSS};
use axum::{
  extract::Query,
  response::{Html, IntoResponse},
};
use serde::{Deserialize, Serialize};
pub mod frame;

#[derive(
  Default, Debug, Clone, Copy, Serialize, Deserialize,
)]
pub enum ViewMode {
  #[default]
  #[serde(alias = "daytime")]
  DayTime,
  #[serde(alias = "night")]
  Night,
}
impl std::fmt::Display for ViewMode {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    f.write_str(match self {
      ViewMode::DayTime => "daytime",
      ViewMode::Night => "night",
    })
  }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct IsSelected(bool);
pub struct IsSelectedVisitor;
impl<'de> serde::de::Visitor<'de> for IsSelectedVisitor {
  type Value = IsSelected;

  fn expecting(
    &self,
    formatter: &mut std::fmt::Formatter,
  ) -> std::fmt::Result {
    formatter.write_str("struct IsSelected")
  }

  fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(match v.trim() {
      "on" => IsSelected(true),
      _ => IsSelected(false),
    })
  }
}
impl<'de> serde::Deserialize<'de> for IsSelected {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_str(IsSelectedVisitor)
  }
}
impl Default for IsSelected {
  fn default() -> Self {
    Self(false)
  }
}
impl std::fmt::Display for IsSelected {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    if self.0 {
      f.write_str("checked")?
    }
    Ok(())
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MainArgs {
  #[serde(alias = "view-mode", default)]
  pub view_mode: ViewMode,
  #[serde(default)]
  pub maximize: IsSelected,
  #[serde(default)]
  pub noheader: IsSelected,
  #[serde(default)]
  pub notaskbar: IsSelected,
  #[serde(default)]
  pub invframe: IsSelected,
  #[serde(default)]
  pub noframe: IsSelected,
}

pub async fn main_page(
  Query(mq): Query<MainArgs>,
) -> impl IntoResponse {
  let mut buffer = String::new();
  frame::gen_frame(&mut buffer, &mq).unwrap();
  Html(buffer)
}

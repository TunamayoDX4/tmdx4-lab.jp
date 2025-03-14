//! メンテナンスページの実装

use axum::{Router, response::Html, routing::post};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MaintePageConfig {
  pub password_dir: String,
  pub initial_username: String,
  pub initial_pswd: String,
  pub argon2_mem_cost: u32,
  pub argon2_time_cost: u32,
  pub argon2_par_cost: u32,
}
impl Default for MaintePageConfig {
  fn default() -> Self {
    Self {
      password_dir: "mainte-pswd".into(),
      initial_username: "Admin01".into(),
      initial_pswd: "D3fau1tPassw0rd".into(),
      argon2_mem_cost: 4096,
      argon2_time_cost: 1,
      argon2_par_cost: 2,
    }
  }
}

pub(crate) fn mainte_serve() -> Router {
  Router::new().route("/", post(async || Html("")))
}

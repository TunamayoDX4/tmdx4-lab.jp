//! メンテナンスページの実装

use std::fmt::Display;
use std::io::{BufReader, BufWriter};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::{fs::File, io::ErrorKind};

use argon2::PasswordVerifier;
use argon2::password_hash::{self, PasswordHasher};
use argon2::{
  Argon2, Params,
  password_hash::{PasswordHashString, SaltString},
};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::post;
use axum::{Form, Router};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::CONFIG;

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

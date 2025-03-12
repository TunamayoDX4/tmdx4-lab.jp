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
  let p1 = PasswordCache::new();
  let p2 = p1.clone();
  Router::new().route("/", post(move |f| mainte_page(f, p1.clone())))
}

/// メンテページログイン用パスワードのserde用生データ
#[derive(Serialize, Deserialize)]
struct PasswordRawData {
  hash: String,
  is_initial: bool,
}

/// パスワードハッシュ化に関わるエラー
#[derive(Debug)]
enum PasswordDataError {
  /// Not Foundを除くファイルIOのエラー
  FileIOError(std::io::Error),
  /// Argon2の初期化処理時に発生するエラー
  Argon2Error(argon2::Error),
  /// パスワードハッシュ生成時に発生するエラー
  InitialHashGenError(password_hash::Error),
  /// PasswordHashString生成時に発生するエラー(PHC文字列不正など)
  PasswordHashStringGenError(password_hash::Error),
  /// MsgPackのデコード時に発生するエラー
  MsgPackDecodeError(rmp_serde::decode::Error),
  /// MsgPackのエンコード時に発生するエラー
  MsgPackEncodeError(rmp_serde::encode::Error),
  /// password_hashに関する広範なエラー
  PasswordHashError(password_hash::Error),
  /// 不正なユーザ名
  InvalidUserName,
}
impl Display for PasswordDataError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    <Self as std::fmt::Debug>::fmt(&self, f)
  }
}
impl std::error::Error for PasswordDataError {}
impl From<std::io::Error> for PasswordDataError {
  fn from(value: std::io::Error) -> Self {
    Self::FileIOError(value)
  }
}

/// パスワード間違えてるよ
#[derive(Debug)]
struct PasswordInvalid;

#[derive(Debug, Deserialize, Serialize)]
struct UserName(String);
impl UserName {
  const INVALID_USERNAME_STRING: &'static [char] = &[
    '\\', '/', ':', ';', '%', '*', '"', '\'', '?', '!', '|', '&', '<', '>',
  ];
  pub fn as_str(&self) -> &str {
    &self.0
  }
}
impl ToString for UserName {
  fn to_string(&self) -> String {
    self.0.clone()
  }
}
impl FromStr for UserName {
  type Err = &'static str;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.contains(Self::INVALID_USERNAME_STRING) {
      return Err("Invalid charactor with in name");
    }
    Ok(Self(s.split(['/', '\\']).last().unwrap().to_string()))
  }
}

/// メンテページログイン用パスワード
struct PasswordData {
  hash: PasswordHashString,
  is_initial: bool,
}
impl PasswordData {
  pub fn get(user_name: &UserName) -> Result<Self, PasswordDataError> {
    let fp = match File::open(format!(
      "{dir}/{user_name}.bin",
      dir = CONFIG.maintenance_page.password_dir,
      user_name = user_name.as_str()
    )) {
      Ok(fp) => fp,
      Err(e) => match e.kind() {
        ErrorKind::NotFound => {
          let hasher = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            match Params::new(
              CONFIG.maintenance_page.argon2_mem_cost,
              CONFIG.maintenance_page.argon2_time_cost,
              CONFIG.maintenance_page.argon2_par_cost,
              None,
            ) {
              Ok(ok) => ok,
              Err(e) => return Err(PasswordDataError::Argon2Error(e)),
            },
          );
          let salt = SaltString::generate(OsRng);
          let hash: PasswordHashString =
            match hasher.hash_password(CONFIG.maintenance_page.initial_pswd.as_bytes(), &salt) {
              Ok(ok) => ok.into(),
              Err(e) => return Err(PasswordDataError::InitialHashGenError(e)),
            };
          return Ok(Self {
            hash,
            is_initial: true,
          });
        }
        _ => return Err(PasswordDataError::FileIOError(e)),
      },
    };
    let fp = BufReader::new(fp);
    let raw: PasswordRawData =
      rmp_serde::from_read(fp).map_err(|e| PasswordDataError::MsgPackDecodeError(e))?;
    let phc = PasswordHashString::new(&raw.hash)
      .map_err(|e| PasswordDataError::PasswordHashStringGenError(e))?;

    Ok(Self {
      hash: phc,
      is_initial: raw.is_initial,
    })
  }

  pub fn set(&self) -> Result<(), PasswordDataError> {
    let fp = File::create(&CONFIG.maintenance_page.password_file)
      .map_err(|e| PasswordDataError::FileIOError(e))?;
    let mut fp = BufWriter::new(fp);
    rmp_serde::encode::write(
      &mut fp,
      &PasswordRawData {
        hash: self.hash.to_string(),
        is_initial: self.is_initial,
      },
    )
    .map_err(|e| PasswordDataError::MsgPackEncodeError(e))
  }
}

#[derive(Clone)]
pub struct PasswordCache(OnceLock<Arc<RwLock<PasswordData>>>);
impl PasswordCache {
  pub fn new() -> Self {
    Self(OnceLock::new())
  }
  fn get(&self) -> Result<RwLockReadGuard<PasswordData>, PasswordDataError> {
    if let Some(get) = self.0.get() {
      Ok(get.read())
    } else {
      let password = PasswordData::get()?;
      password.set()?;
      Ok(
        self
          .0
          .get_or_init(|| Arc::new(RwLock::new(password)))
          .read(),
      )
    }
  }

  fn set(
    &self,
    old_password: &[u8],
    new_password: &[u8],
  ) -> Result<Result<RwLockReadGuard<PasswordData>, PasswordInvalid>, PasswordDataError> {
    let mut get = if let Some(get) = self.0.get() {
      get.write()
    } else {
      let password = PasswordData::get()?;
      password.set()?;
      self
        .0
        .get_or_init(|| Arc::new(RwLock::new(password)))
        .write()
    };
    let salt = get.hash.salt().unwrap();
    let argon2_instance = Argon2::new(
      argon2::Algorithm::Argon2id,
      argon2::Version::V0x13,
      Params::new(
        CONFIG.maintenance_page.argon2_mem_cost,
        CONFIG.maintenance_page.argon2_time_cost,
        CONFIG.maintenance_page.argon2_par_cost,
        None,
      )
      .map_err(|e| PasswordDataError::Argon2Error(e))?,
    );
    match argon2_instance.verify_password(old_password, &get.hash.password_hash()) {
      Ok(_) => {}
      Err(e) => match e {
        password_hash::Error::Password => {
          return Ok(Err(PasswordInvalid));
        }
        _ => return Err(PasswordDataError::PasswordHashError(e)),
      },
    }
    let new_hash = argon2_instance
      .hash_password(new_password, salt)
      .map_err(|e| PasswordDataError::PasswordHashError(e))?;
    get.hash = new_hash.into();
    get.is_initial = false;
    get.set()?;
    Ok(Ok(RwLockWriteGuard::downgrade(get)))
  }

  fn verify(&self, password: &[u8]) -> Result<bool, PasswordDataError> {
    Ok(
      match Argon2::default().verify_password(password, &self.get()?.hash.password_hash()) {
        Ok(_) => true,
        Err(e) => match e {
          password_hash::Error::Password => false,
          _ => return Err(PasswordDataError::PasswordHashError(e)),
        },
      },
    )
  }
}

#[derive(Debug, Deserialize, Serialize)]
struct MaintePageFormObject {
  password: String,
  #[serde(alias = "new-password")]
  new_password: Option<String>,
  #[serde(alias = "new-password-verify")]
  new_password_verify: Option<String>,
  diary_title: Option<String>,
  diary_text: Option<String>,
}

async fn mainte_page(
  Form(form): Form<MaintePageFormObject>,
  password: PasswordCache,
) -> Result<impl IntoResponse, impl IntoResponse> {
  if !password.verify(form.password.trim().as_bytes()).unwrap() {
    Err(StatusCode::FORBIDDEN)
  } else {
    let (p, is_ok) = match (
      form.new_password.map(|s| s),
      form.new_password_verify.map(|s| s),
    ) {
      (Some(np), Some(np_v)) if np.trim() == np_v.trim() => {
        let p = password
          .set(form.password.trim().as_bytes(), np.trim().as_bytes())
          .unwrap()
          .unwrap();
        (p, true)
      }
      (Some(_), None) | (None, Some(_)) => return Err(StatusCode::BAD_REQUEST),
      (None, None) => (password.get().unwrap(), true),
      (Some(_), Some(_)) => (password.get().unwrap(), false),
    };
    if p.is_initial {
      // 初期パスなら変更を要求する
      Ok(Html(format!(
        "<!doctype html><html><head></head><body>\
          <h1>パスワードの変更</h1>\
          <p>安全の為、初期パスワードからの変更をお願いします</p>\
          {0}
          <form action=\"\" method=\"POST\">\
            <label><input type=\"password\" name=\"password\" width=\"32\">: 現行のパスワード</label><br>\
            <label><input type=\"password\" name=\"new-password\" width=\"32\">: 新しいパスワード</label><br>\
            <label><input type=\"password\" name=\"new-password-verify\" width=\"32\">: 新しいパスワード(確認)</label><br>\
            <input type=\"submit\" value=\"変更のリクエスト\">\
          </form>\
        </body></html>",
      if !is_ok {
        "<span style=\"color: red\">新しいパスワードが一致しません。入力内容をご確認ください…</style>"
      } else { "" }
      )))
    } else {
      // そうでないならメンテページへ
      Ok(Html(format!(
        "<!doctype html><html>\
          <head>\
            <title>メンテページ</title>\
          </head>\
          <body>\
            <h1>メンテページへようこそ</h1>\
            <a href=\"/\">トップページへ</a>\
            <hr>\
            <section id=\"put-diary\">\
              <form method=\"POST\" action=\"/mainte/put-diary\">\
                <label><input type=\"password\" name=\"password\">: パスワード</label><br>\
                <label>題名: <br><input type=\"text\" name=\"diary-title\" width=\"96\" value=\"昨日のお昼\"></label><br>\
                <label>本文: <br><textarea name=\"diary-text\" rows=\"32\" cols=\"96\"></textarea></label><br>\
                <input type=\"submit\" value=\"アップロード\">\
              </form>\
            </section>\
            <hr>\
            <section id=\"change-password\">\
              <h2>メンテ画面のパスワードの変更</h2>\
              <form method=\"POST\" action=\"\" >\
                <label><input type=\"password\" name=\"password\" width=\"32\">: 現行のパスワード</label><br>\
                <label><input type=\"password\" name=\"new-password\" width=\"32\">: 新しいパスワード</label><br>\
                <label><input type=\"password\" name=\"new-password-verify\" width=\"32\">: 新しいパスワード(確認)</label><br>\
                <input type=\"submit\" value=\"変更のリクエスト\">\
              </form>\
            </section>\
          </body>\
        </html>",
      )))
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
struct PutDiaryObject {
  password: String,
  #[serde(alias = "diary-title")]
  diary_title: String,
  #[serde(alias = "diary-text")]
  diary_text: String,
}

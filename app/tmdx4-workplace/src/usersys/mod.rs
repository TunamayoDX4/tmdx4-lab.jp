//! ユーザシステムの実装

use argon2::{PasswordHash, password_hash::PasswordHashString};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{
  io::{Read, Write},
  str::FromStr,
};

/// ユーザID生成失敗エラー
#[derive(Debug)]
pub enum UserDataError {
  /// 不正な文字列入ってんだけど！
  InvalidCharInIdent,

  /// ユーザデータの読み込み時のエラー
  UserDataLoadError(std::io::Error),

  /// ユーザデータの書き込み時のエラー
  UserDataSaveError(std::io::Error),

  /// パスワードハッシュの読み取り時のエラー
  PasswordHashReadError(argon2::password_hash::Error),

  /// MsgPackのデコードのエラー
  MPackDecodeError(rmp_serde::decode::Error),

  /// MsgPackのエンコードのエラー
  MPackEncodeError(rmp_serde::encode::Error),

  /// ロジックに起因する回復不能なバグ
  LogicError(Box<dyn std::error::Error>),
}
impl std::fmt::Display for UserDataError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::InvalidCharInIdent => f.write_str("Invalid charactor with in userid"),
      Self::UserDataLoadError(e) => f.write_fmt(format_args!("User data loading error: {e}")),
      Self::UserDataSaveError(e) => f.write_fmt(format_args!("User data saving error: {e}")),
      Self::PasswordHashReadError(e) => {
        f.write_fmt(format_args!("Password hash reading error: {e}"))
      }
      Self::MPackDecodeError(e) => f.write_fmt(format_args!("Message pack decode error: {e}")),
      Self::MPackEncodeError(e) => f.write_fmt(format_args!("Message pack encode error: {e}")),
      Self::LogicError(error) => f.write_fmt(format_args!(
        "Uncoverable error. please report to author: {error}"
      )),
    }
  }
}

/// ユーザ識別キー(所謂ID)
/// ユーザが入力したアルファベット配列をハッシュにして使う
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserIdent(#[serde(with = "serde_bytes")] [u8; 32]);
impl UserIdent {
  pub const INVALID_CHARS: &'static [char] =
    &['/', '\\', '*', '+', '?', '.', ',', '~', '^', '<', '>'];

  pub fn generate(s: &str) -> Result<Self, UserDataError> {
    if s.contains(Self::INVALID_CHARS) {
      return Err(UserDataError::InvalidCharInIdent);
    }
    let mut hasher = Sha3_256::new();
    hasher.update(s.trim().as_bytes());
    let mut dst = [0u8; 32];
    (&mut dst[..])
      .write(hasher.finalize().as_slice())
      .map_err(|e| UserDataError::LogicError(Box::from(e)))?;
    Ok(Self(dst))
  }
}

/// ユーザデータ
pub struct UserData {
  ident: UserIdent,
  pswd_hash: PasswordHashString,
}
impl UserData {
  pub fn load(name: &str, load_dir: &str) -> Result<Self, UserDataError> {
    let ident = match UserIdent::generate(name) {
      Ok(ident) => ident,
      Err(e) => return Err(e),
    };
    let mut buffer = String::with_capacity(load_dir.len() + 21);
    buffer += load_dir;
    buffer.push('/');
    for ch in ident
      .0
      .iter()
      .flat_map(|b| [b & 0xF0, b & 0x0F].into_iter())
      .filter_map(|b| char::from_digit(b as u32, 16))
    {
      buffer.push(ch);
    }
    buffer += ".bin";
    let mut rdr = std::io::BufReader::new(
      std::fs::File::open(&buffer).map_err(|e| UserDataError::UserDataLoadError(e))?,
    );
    buffer.clear();
    rdr
      .read_to_string(&mut buffer)
      .map_err(|e| UserDataError::UserDataLoadError(e))?;
    let pswd_hash =
      PasswordHashString::new(&buffer).map_err(|e| UserDataError::PasswordHashReadError(e))?;

    Ok(Self { ident, pswd_hash })
  }
  pub fn save(&self, load_dir: &str) -> Result<(), UserDataError> {
    if !std::fs::exists(load_dir).map_err(|e| UserDataError::UserDataSaveError(e))? {
      std::fs::create_dir_all(load_dir).map_err(|e| UserDataError::UserDataSaveError(e))?
    }
    let mut buffer = String::with_capacity(load_dir.len() + 21);
    buffer += load_dir;
    buffer.push('/');
    for ch in self
      .ident
      .0
      .iter()
      .flat_map(|b| [b & 0xF0, b & 0xF0].into_iter())
      .filter_map(|b| char::from_digit(b as u32, 16))
    {
      buffer.push(ch);
    }
    todo!()
  }
}

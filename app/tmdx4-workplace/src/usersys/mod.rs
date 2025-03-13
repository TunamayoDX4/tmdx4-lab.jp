//! ユーザシステムの実装

use argon2::{PasswordHasher, PasswordVerifier, password_hash::PasswordHashString};
use rand::{
  SeedableRng,
  rngs::{OsRng, ThreadRng},
};
use rand_chacha::{ChaCha20Core, ChaCha20Rng};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{
  cell::{LazyCell, RefCell},
  io::{Read, Write},
  ops::Deref,
  str::FromStr,
};

thread_local! {
  static PRNG: LazyCell<RefCell<ChaCha20Rng>> = LazyCell::new(|| RefCell::new(ChaCha20Rng::from_entropy()));
}

/// ユーザID生成失敗エラー
#[derive(Debug)]
pub enum UserDataError {
  /// 不正な文字列入ってんだけど！
  InvalidCharInIdent,

  /// ユーザデータの読み込み時のエラー
  UserDataLoadError(std::io::Error),

  /// ユーザデータの書き込み時のエラー
  UserDataSaveError(std::io::Error),

  /// パスワードハッシュのエラー
  PasswordHashError(argon2::password_hash::Error),

  /// Argon2におけるハッシュ生成に関するエラー
  Argon2Error(argon2::Error),

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
      Self::PasswordHashError(e) => f.write_fmt(format_args!("Password hash reading error: {e}")),
      Self::Argon2Error(e) => f.write_fmt(format_args!("Argon2 hasher error: {e}")),
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
  /// ユーザ名にこの文字入れたらNG(ファイル名に使うので…)
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

/// ユーザデータについてのコンフィグ
#[derive(Serialize, Deserialize, Debug)]
pub struct UserDataConfig {
  /// セキュリティ関連データのパス
  pub sec_data_path: String,

  /// 各ユーザ用拡張データのパス
  pub user_data_path: String,

  /// Argon2のメモリコスト
  pub argon2_m_cost: u32,

  /// Argon2の時間的コスト
  pub argon2_t_cost: u32,

  /// Argon2の並列化コスト
  pub argon2_p_cost: u32,
}
impl UserDataConfig {
  pub fn init_argon2_param(&self) -> Result<argon2::Params, argon2::Error> {
    argon2::Params::new(
      self.argon2_m_cost,
      self.argon2_t_cost,
      self.argon2_p_cost,
      None,
    )
  }
}
impl Default for UserDataConfig {
  fn default() -> Self {
    Self {
      sec_data_path: "./user_data/secure".into(),
      user_data_path: "./user_data/user_data".into(),
      argon2_m_cost: 4096,
      argon2_t_cost: 1,
      argon2_p_cost: 2,
    }
  }
}

/// ユーザデータ
pub struct UserData<D>
where
  D: Serialize + for<'de> Deserialize<'de> + Send + Sync + Sized,
{
  ident: UserIdent,
  pswd_hash: PasswordHashString,
  user_data: D,
}
impl<D> UserData<D>
where
  D: Serialize + for<'de> Deserialize<'de> + Send + Sync + Sized,
{
  pub fn new(
    name: &str,
    pswd: &str,
    user_data: D,
    configure: &UserDataConfig,
  ) -> Result<Self, UserDataError> {
    let ident = UserIdent::generate(name)?;
    let argon2 = argon2::Argon2::new(
      argon2::Algorithm::Argon2id,
      argon2::Version::V0x13,
      configure
        .init_argon2_param()
        .map_err(|e| UserDataError::Argon2Error(e))?,
    );
    let salt =
      PRNG.with(|prng| argon2::password_hash::SaltString::generate(&mut *prng.borrow_mut()));
    let hash = argon2
      .hash_password(pswd.trim().as_bytes(), salt.as_salt())
      .map_err(|e| UserDataError::PasswordHashError(e))?;
    let hash = PasswordHashString::from(hash);
    Ok(Self {
      ident,
      pswd_hash: hash,
      user_data,
    })
  }
  pub fn verify(
    &mut self,
    pswd: &str,
    new_pswd: Option<&str>,
    new_data: Option<D>,
    configure: &UserDataConfig,
  ) -> Result<bool, UserDataError> {
    let pb = pswd.trim().as_bytes();
    let argon2 = argon2::Argon2::new(
      argon2::Algorithm::Argon2id,
      argon2::Version::V0x13,
      configure
        .init_argon2_param()
        .map_err(|e| UserDataError::Argon2Error(e))?,
    );
    match argon2.verify_password(pb, &self.pswd_hash.password_hash()) {
      Ok(_) => {}
      Err(argon2::password_hash::Error::Password) => return Ok(false),
      Err(e) => return Err(UserDataError::PasswordHashError(e)),
    }
    let salt =
      PRNG.with(|prng| argon2::password_hash::SaltString::generate(&mut *prng.borrow_mut()));
    let hash = argon2
      .hash_password(
        new_pswd.map(|np| np.trim().as_bytes()).unwrap_or(pb),
        salt.as_salt(),
      )
      .map_err(|e| UserDataError::PasswordHashError(e))?;
    self.pswd_hash = PasswordHashString::from(hash);
    if let Some(nd) = new_data {
      self.user_data = nd
    }
    self.save(&configure)?;
    Ok(true)
  }

  pub fn load(name: &str, configure: &UserDataConfig) -> Result<Self, UserDataError> {
    let ident = match UserIdent::generate(name) {
      Ok(ident) => ident,
      Err(e) => return Err(e),
    };
    let mut buffer = String::with_capacity(configure.sec_data_path.len() + 21);
    let mut buffer2 = String::with_capacity(configure.user_data_path.len() + 21);
    buffer += &configure.sec_data_path;
    buffer2 += &configure.user_data_path;
    buffer.push('/');
    buffer2.push('/');
    for ch in ident
      .0
      .iter()
      .flat_map(|b| [b & 0xF0, b & 0x0F].into_iter())
      .filter_map(|b| char::from_digit(b as u32, 16))
    {
      buffer.push(ch);
      buffer2.push(ch);
    }
    buffer += ".bin";
    buffer2 += ".bin";
    let mut rdr = std::io::BufReader::new(
      std::fs::File::open(&buffer).map_err(|e| UserDataError::UserDataLoadError(e))?,
    );
    buffer.clear();
    rdr
      .read_to_string(&mut buffer)
      .map_err(|e| UserDataError::UserDataLoadError(e))?;
    let pswd_hash =
      PasswordHashString::new(&buffer).map_err(|e| UserDataError::PasswordHashError(e))?;
    let rdr = std::io::BufReader::new(
      std::fs::File::open(&buffer2).map_err(|e| UserDataError::UserDataLoadError(e))?,
    );
    let user_data = rmp_serde::from_read(rdr).map_err(|e| UserDataError::MPackDecodeError(e))?;

    Ok(Self {
      ident,
      pswd_hash,
      user_data,
    })
  }

  pub fn save(&self, configure: &UserDataConfig) -> Result<(), UserDataError> {
    if !std::fs::exists(&configure.sec_data_path)
      .map_err(|e| UserDataError::UserDataSaveError(e))?
    {
      std::fs::create_dir_all(&configure.sec_data_path)
        .map_err(|e| UserDataError::UserDataSaveError(e))?
    }
    let mut buffer = String::with_capacity(&configure.sec_data_path.len() + 21);
    let mut buffer2 = String::with_capacity(&configure.user_data_path.len() + 21);
    buffer += &configure.sec_data_path;
    buffer2 += &configure.user_data_path;
    buffer.push('/');
    buffer2.push('/');
    for ch in self
      .ident
      .0
      .iter()
      .flat_map(|b| [b & 0xF0, b & 0x0F].into_iter())
      .filter_map(|b| char::from_digit(b as u32, 16))
    {
      buffer.push(ch);
      buffer2.push(ch);
    }
    buffer += ".bin";
    buffer2 += ".bin";
    let mut wrt = std::io::BufWriter::new(
      std::fs::File::create(buffer).map_err(|e| UserDataError::UserDataSaveError(e))?,
    );
    wrt
      .write(self.pswd_hash.as_str().as_bytes())
      .map_err(|e| UserDataError::UserDataSaveError(e))?;
    let mut wrt = std::io::BufWriter::new(
      std::fs::File::create(buffer2).map_err(|e| UserDataError::UserDataSaveError(e))?,
    );
    rmp_serde::encode::write(&mut wrt, &self.user_data)
      .map_err(|e| UserDataError::MPackEncodeError(e))?;
    Ok(())
  }
}

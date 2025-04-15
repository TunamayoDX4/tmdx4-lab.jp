//! ユーザシステムの実装

use argon2::{
  PasswordHasher, password_hash::PasswordHashString,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{
  cell::{LazyCell, RefCell},
  fmt::Write as FmtWrite,
  io::{Read, Write},
};

thread_local! {
  static PRNG: LazyCell<RefCell<ChaCha20Rng>> = LazyCell::new(
    || RefCell::new(ChaCha20Rng::from_entropy())
  );
}

/// ユーザID生成失敗エラー
#[derive(Debug)]
pub enum UserDataError {
  /// ユーザIDが競合してる！被ってる！
  UserIDConflict,

  /// 不正な文字列入ってんだけど！
  InvalidCharInIdent,

  /// ユーザデータの読み込みができなかったよ
  UserDataLoadCannot {
    sec_data: Option<std::io::Error>,
    user_data: Option<std::io::Error>,
  },

  /// ユーザデータの初期化が出来なかったよ
  UserDataInitializeError(Box<dyn std::error::Error>),

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
impl UserDataError {
  const NOT_ERR_STR: &'static str = "nothing error";
}
impl std::fmt::Display for UserDataError {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    match self {
      Self::UserIDConflict => f.write_str("Userid Conflicted."),
      Self::InvalidCharInIdent => {
        f.write_str("Invalid charactor with in userid")
      }
      Self::UserDataLoadCannot {
        sec_data,
        user_data,
      } => f.write_fmt(format_args!(
        "User data cannot load. please send a report.: {{\n\
          secure_data: {sec_data} \n\
          user_data: {user_data} \n\
        }}",
        sec_data = if let Some(e) = sec_data.as_ref() {
          e as &dyn std::fmt::Display
        } else {
          &Self::NOT_ERR_STR
        },
        user_data = if let Some(e) = user_data.as_ref() {
          e as &dyn std::fmt::Display
        } else {
          &Self::NOT_ERR_STR
        }
      )),
      Self::UserDataInitializeError(e) => f.write_fmt(
        format_args!("User data initialize error: {e}"),
      ),
      Self::UserDataLoadError(e) => {
        f.write_fmt(format_args!("User data loading error: {e}"))
      }
      Self::UserDataSaveError(e) => {
        f.write_fmt(format_args!("User data saving error: {e}"))
      }
      Self::PasswordHashError(e) => f.write_fmt(format_args!(
        "Password hash reading error: {e}"
      )),
      Self::Argon2Error(e) => {
        f.write_fmt(format_args!("Argon2 hasher error: {e}"))
      }
      Self::MPackDecodeError(e) => f.write_fmt(format_args!(
        "Message pack decode error: {e}"
      )),
      Self::MPackEncodeError(e) => f.write_fmt(format_args!(
        "Message pack encode error: {e}"
      )),
      Self::LogicError(error) => f.write_fmt(format_args!(
        "Uncoverable error. please report to author: {error}"
      )),
    }
  }
}
impl std::error::Error for UserDataError {}

/// ユーザ識別キー(所謂ID)
/// ユーザが入力したアルファベット配列をハッシュにして使う
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct UserIdent(#[serde(with = "serde_bytes")] [u8; 32]);
impl std::fmt::Display for UserIdent {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    self
      .0
      .iter()
      .flat_map(|c| [c & 0xF0, c & 0x0F].into_iter())
      .filter_map(|c| char::from_digit(c as u32, 16))
      .fold(std::fmt::Result::Ok(()), |i, v| match i {
        Ok(_) => f.write_char(v),
        Err(e) => Err(e),
      })
  }
}
impl UserIdent {
  /// ユーザ名にこの文字入れたらNG(ファイル名に使うので…)
  pub const INVALID_CHARS: &'static [char] = &[
    '/', '\\', '*', '+', '?', '.', ',', '~', '^', '<', '>', '"',
    '\'',
  ];

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

  pub fn iter_hex(&self, f: impl FnMut(char)) {
    self
      .0
      .iter()
      .flat_map(|c| [c & 0xF0, c & 0x0F].into_iter())
      .filter_map(|c| char::from_digit(c as u32, 16))
      .for_each(f);
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
  pub fn init_argon2_param(
    &self,
  ) -> Result<argon2::Params, argon2::Error> {
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
  pub fn ident(&self) -> &UserIdent {
    &self.ident
  }

  pub fn user_data(&self) -> &D {
    &self.user_data
  }

  pub fn user_data_mut(&mut self) -> &mut D {
    &mut self.user_data
  }

  pub fn check_users_exist(
    configure: &UserDataConfig,
  ) -> Result<bool, Box<dyn std::error::Error>> {
    match (
      std::fs::read_dir(&configure.sec_data_path),
      std::fs::read_dir(&configure.user_data_path),
    ) {
      (Ok(s), Ok(u)) => Ok(s.count() != 0 && u.count() != 0),
      (Err(se), Err(ue)) => {
        let (sek, uek) = (se.kind(), ue.kind());
        match (sek, uek) {
          (
            std::io::ErrorKind::NotFound,
            std::io::ErrorKind::NotFound,
          ) => return Ok(false),
          _ => return Err(Box::from(se)),
        }
      }
      (Err(e), _) | (_, Err(e)) => return Err(Box::from(e)),
    }
  }

  pub fn check_exist(
    id: &str,
    configure: &UserDataConfig,
  ) -> Result<bool, Box<dyn std::error::Error>> {
    // パスを生成する
    let ident = UserIdent::generate(id)?;
    let sec_data_path_len = configure.sec_data_path.len() + 24;
    let mut sec_data_path =
      String::with_capacity(sec_data_path_len);
    sec_data_path += configure.sec_data_path.as_str();
    sec_data_path.push('/');
    let user_data_path_len = configure.user_data_path.len() + 24;
    let mut user_data_path =
      String::with_capacity(user_data_path_len);
    user_data_path += configure.user_data_path.as_str();
    user_data_path.push('/');
    ident.iter_hex(|c| {
      sec_data_path.push(c);
      user_data_path.push(c);
    });
    sec_data_path += ".bin";
    user_data_path += ".bin";
    let r = std::fs::exists(&sec_data_path)?
      && std::fs::exists(&user_data_path)?;
    Ok(r)
  }

  pub fn load(
    id: &str,
    pswd: &str,
    new_pswd: Option<&str>,
    configure: &UserDataConfig,
    user_data_init_func: impl FnOnce() -> Result<
      D,
      Box<dyn std::error::Error>,
    >,
  ) -> Result<Option<Self>, UserDataError> {
    // パスを生成する
    let ident = UserIdent::generate(id)?;
    let sec_data_path_len = configure.sec_data_path.len() + 24;
    let mut sec_data_path =
      String::with_capacity(sec_data_path_len);
    sec_data_path += configure.sec_data_path.as_str();
    sec_data_path.push('/');
    let user_data_path_len = configure.user_data_path.len() + 24;
    let mut user_data_path =
      String::with_capacity(user_data_path_len);
    user_data_path += configure.user_data_path.as_str();
    user_data_path.push('/');
    ident.iter_hex(|c| {
      sec_data_path.push(c);
      user_data_path.push(c);
    });
    sec_data_path += ".bin";
    user_data_path += ".bin";

    // セキュリティデータを読み込む
    // NotFoundならNone, そうでないならデータありとする
    let sec_data = match std::fs::File::open(&sec_data_path) {
      Ok(rdr) => Some(std::io::BufReader::new(rdr)),
      Err(e) => match e.kind() {
        std::io::ErrorKind::NotFound => None,
        _ => return Err(UserDataError::UserDataLoadError(e)),
      },
    };

    // ユーザデータはバッファを開けるために先に読み込みます。リカバ不能のエラーなら終了。
    // ヒープを使いまわしたいのでバッファは捨てない。

    // ユーザデータを読み込む
    // NotFoundならNone, そうでないならデータありとする
    let user_data = match std::fs::File::open(&user_data_path)
      .map(|fp| std::io::BufReader::new(fp))
    {
      Ok(rdr) => Some(rdr),
      Err(e) => match e.kind() {
        std::io::ErrorKind::NotFound => None,
        _ => return Err(UserDataError::UserDataLoadError(e)),
      },
    };

    // セキュリティデータバッファを使いまわす。
    let mut secure_data_buffer = {
      let mut buf = user_data_path;
      buf.clear();
      buf
    };

    // パスワードの比較処理の開始
    let argon2 = argon2::Argon2::new(
      argon2::Algorithm::Argon2id,
      argon2::Version::V0x13,
      configure
        .init_argon2_param()
        .map_err(|e| UserDataError::Argon2Error(e))?,
    );

    // セキュリティデータのデシリアライズ
    if let Some(mut sec_data) = sec_data {
      // ファイルがあれば内容を読み取る
      sec_data
        .read_to_string(&mut secure_data_buffer)
        .map_err(|e| UserDataError::UserDataLoadError(e))?;
    } else {
      // ファイルがないならダミーを作る
      secure_data_buffer
        .write_fmt(format_args!(
          "\
            $argon2id\
            $v=19\
            $m={0},t={1},p={2}\
            $xXrE6TTlFREZbmJDW95cKQ\
            $Puy1C+9fn8eYyq256f7C14QAPBVI40qPwqmST+HB8aw\
          ",
          configure.argon2_m_cost,
          configure.argon2_t_cost,
          configure.argon2_p_cost,
        ))
        .unwrap();
    };

    // ハッシュ処理
    let hash = argon2::password_hash::PasswordHash::new(
      &secure_data_buffer,
    )
    .map_err(|e| UserDataError::PasswordHashError(e))?;
    let salt = hash.salt.unwrap();

    let comp_hash =
      argon2::password_hash::PasswordHash::generate(
        argon2.clone(),
        pswd.trim(),
        salt,
      )
      .map_err(|e| UserDataError::PasswordHashError(e))?;

    // ハッシュ比較して違ったらOk(None)
    if hash != comp_hash {
      return Ok(None);
    }

    // ソルトを作る
    let salt = PRNG.with(|prng| {
      argon2::password_hash::SaltString::generate(
        &mut *prng.borrow_mut(),
      )
    });

    // パスワードの再ハッシュ
    let pswd_hash =
      argon2::password_hash::PasswordHash::generate(
        argon2,
        new_pswd.unwrap_or(pswd).trim(),
        salt.as_salt(),
      )
      .unwrap()
      .serialize();
    // 再ハッシュしたのを書き込む
    std::io::BufWriter::new(
      std::fs::File::create(&sec_data_path)
        .map_err(|e| UserDataError::UserDataSaveError(e))?,
    )
    .write(pswd_hash.as_bytes())
    .map_err(|e| UserDataError::UserDataSaveError(e))?;

    // ユーザデータの読み込み
    let user_data = match user_data
      .map(|rdr| rmp_serde::from_read(rdr))
    {
      Some(Err(e)) => Err(UserDataError::MPackDecodeError(e)),
      Some(Ok(v)) => Ok(v),
      None => user_data_init_func()
        .map_err(|e| UserDataError::UserDataInitializeError(e)),
    }?;

    Ok(Some(Self {
      ident,
      pswd_hash,
      user_data,
    }))
  }
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
    let salt = PRNG.with(|prng| {
      argon2::password_hash::SaltString::generate(
        &mut *prng.borrow_mut(),
      )
    });
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

  pub fn save(
    &self,
    configure: &UserDataConfig,
  ) -> Result<(), UserDataError> {
    if !std::fs::exists(&configure.sec_data_path)
      .map_err(|e| UserDataError::UserDataSaveError(e))?
    {
      std::fs::create_dir_all(&configure.sec_data_path)
        .map_err(|e| UserDataError::UserDataSaveError(e))?
    }
    if !std::fs::exists(&configure.user_data_path)
      .map_err(|e| UserDataError::UserDataSaveError(e))?
    {
      std::fs::create_dir_all(&configure.user_data_path)
        .map_err(|e| UserDataError::UserDataSaveError(e))?
    }
    let mut buffer =
      String::with_capacity(&configure.sec_data_path.len() + 21);
    let mut buffer2 = String::with_capacity(
      &configure.user_data_path.len() + 21,
    );
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
      std::fs::File::create(buffer)
        .map_err(|e| UserDataError::UserDataSaveError(e))?,
    );
    wrt
      .write(self.pswd_hash.as_str().as_bytes())
      .map_err(|e| UserDataError::UserDataSaveError(e))?;
    let mut wrt = std::io::BufWriter::new(
      std::fs::File::create(buffer2)
        .map_err(|e| UserDataError::UserDataSaveError(e))?,
    );
    rmp_serde::encode::write(&mut wrt, &self.user_data)
      .map_err(|e| UserDataError::MPackEncodeError(e))?;
    Ok(())
  }
}

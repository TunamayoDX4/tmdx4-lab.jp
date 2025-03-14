//! ユーザシステムの実装

use argon2::{
  PasswordHasher, PasswordVerifier,
  password_hash::PasswordHashString,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{
  cell::{LazyCell, RefCell},
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

/// ユーザ識別キー(所謂ID)
/// ユーザが入力したアルファベット配列をハッシュにして使う
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct UserIdent(#[serde(with = "serde_bytes")] [u8; 32]);
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

#[cfg(test)]
mod test {
  use serde::{Deserialize, Serialize};

  use super::{UserData, UserDataConfig};

  #[derive(Serialize, Deserialize, PartialEq, Eq)]
  struct SampleUserData {
    shown_name: String,
    email_addr: String,
    age: u32,
  }

  fn get_sample(pat: u32) -> Option<SampleUserData> {
    match pat {
      0 => Some(SampleUserData {
        shown_name: "たかし".into(),
        email_addr: "takashi@example.com".into(),
        age: 23,
      }),
      _ => None,
    }
  }

  #[test]
  fn testing() {
    let configure: UserDataConfig = UserDataConfig {
      sec_data_path: "test.secure".into(),
      user_data_path: "test.usdt".into(),
      argon2_m_cost: 512,
      argon2_t_cost: 1,
      argon2_p_cost: 1,
    };

    let ud = UserData::new(
      "takashi",
      "takashi2354",
      get_sample(0).unwrap(),
      &configure,
    )
    .unwrap();

    ud.save(&configure).unwrap();

    let mut ud =
      UserData::<SampleUserData>::load("takashi", &configure)
        .unwrap();
    assert!(
      ud.verify("takashi2354", None, None, &configure).unwrap()
    );
    assert!(
      !ud.verify("takashi1234", None, None, &configure).unwrap()
    )
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
  pub fn load_renewed(
    id: &str,
    pswd: &str,
    configure: &UserDataConfig,
  ) -> Result<Option<()>, UserDataError> {
    let ident = UserIdent::generate(id)?;
    let sec_data_path_len = configure.sec_data_path.len() + 24;
    let mut sec_data_path =
      String::with_capacity(sec_data_path_len);
    sec_data_path += configure.sec_data_path.as_str();
    let user_data_path_len = configure.user_data_path.len() + 24;
    let mut user_data_path =
      String::with_capacity(user_data_path_len);
    user_data_path += configure.user_data_path.as_str();
    ident.iter_hex(|c| {
      sec_data_path.push(c);
      user_data_path.push(c);
    });
    sec_data_path += ".bin";
    user_data_path += ".bin";
    let (fp_sec_data, fp_user_data) = match (
      std::fs::File::open(&sec_data_path),
      std::fs::File::open(&user_data_path),
    ) {
      (Ok(sfp), Ok(ufp)) => (sfp, ufp),
      (Err(es), Err(eu)) => match (es.kind(), eu.kind()) {
        (
          std::io::ErrorKind::NotFound,
          std::io::ErrorKind::NotFound,
        ) => return Ok(None),
        (_, _) => {
          return Err(UserDataError::UserDataLoadCannot {
            sec_data: Some(es),
            user_data: Some(eu),
          });
        }
      },
      (Err(es), Ok(_)) => {
        return Err(UserDataError::UserDataLoadCannot {
          sec_data: Some(es),
          user_data: None,
        });
      }
      (Ok(_), Err(eu)) => {
        return Err(UserDataError::UserDataLoadCannot {
          sec_data: None,
          user_data: Some(eu),
        });
      }
    };
    let (mut rdr_sec_data, mut rdr_user_data) = (
      std::io::BufReader::new(fp_sec_data),
      std::io::BufReader::new(fp_user_data),
    );

    // バッファの使いまわし
    let mut buffer_sec_data = {
      let mut buffer = sec_data_path;
      buffer.clear();
      buffer
    };
    rdr_sec_data.read_to_string(&mut buffer_sec_data);

    // バッファの使いまわし
    let buffer_user_data = {
      let mut bytes = user_data_path.into_bytes();
      bytes.clear();
      bytes
    };

    Ok(None)
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
    match argon2
      .verify_password(pb, &self.pswd_hash.password_hash())
    {
      Ok(_) => {}
      Err(argon2::password_hash::Error::Password) => {
        return Ok(false);
      }
      Err(e) => return Err(UserDataError::PasswordHashError(e)),
    }
    let salt = PRNG.with(|prng| {
      argon2::password_hash::SaltString::generate(
        &mut *prng.borrow_mut(),
      )
    });
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

  pub fn load(
    name: &str,
    configure: &UserDataConfig,
  ) -> Result<Self, UserDataError> {
    let ident = match UserIdent::generate(name) {
      Ok(ident) => ident,
      Err(e) => return Err(e),
    };
    let mut buffer =
      String::with_capacity(configure.sec_data_path.len() + 21);
    let mut buffer2 =
      String::with_capacity(configure.user_data_path.len() + 21);
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
      std::fs::File::open(&buffer)
        .map_err(|e| UserDataError::UserDataLoadError(e))?,
    );
    buffer.clear();
    rdr
      .read_to_string(&mut buffer)
      .map_err(|e| UserDataError::UserDataLoadError(e))?;
    let pswd_hash = PasswordHashString::new(&buffer)
      .map_err(|e| UserDataError::PasswordHashError(e))?;
    let rdr = std::io::BufReader::new(
      std::fs::File::open(&buffer2)
        .map_err(|e| UserDataError::UserDataLoadError(e))?,
    );
    let user_data = rmp_serde::from_read(rdr)
      .map_err(|e| UserDataError::MPackDecodeError(e))?;

    Ok(Self {
      ident,
      pswd_hash,
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

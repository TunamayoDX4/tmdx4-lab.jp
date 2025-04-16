//! メンテナンスページの実装

use std::path::PathBuf;

use axum::{
  Form, Router,
  http::StatusCode,
  response::{Html, IntoResponse},
  routing::post,
};
use serde::{Deserialize, Serialize};

use crate::usersys;
pub mod page_gen;
pub const MAINTE_CSS: &'static str =
  include_str!("../styles/mainte.css");

#[derive(Deserialize, Serialize)]
pub struct MaintePageConfig {
  pub password_dir: String,
  pub initial_username: String,
  pub initial_pswd: String,
  pub pswd_len_min: usize,
  pub usersys_config: usersys::UserDataConfig,
}
impl MaintePageConfig {
  pub fn is_default_user<T>(
    &self,
    userdata: &crate::usersys::UserData<T>,
  ) -> Result<bool, Box<dyn std::error::Error>>
  where
    T: Serialize
      + for<'de> serde::de::Deserialize<'de>
      + Send
      + Sync,
  {
    let user_ident = crate::usersys::UserIdent::generate(
      &self.initial_username,
    )?;
    Ok(*userdata.ident() == user_ident)
  }
}
impl Default for MaintePageConfig {
  fn default() -> Self {
    Self {
      password_dir: "mainte-pswd".into(),
      initial_username: "Admin01".into(),
      initial_pswd: "D3fau1tPassw0rd".into(),
      pswd_len_min: 16,
      usersys_config: usersys::UserDataConfig {
        sec_data_path: "mainte-user-sec".into(),
        user_data_path: "mainte-user-data".into(),
        argon2_m_cost: 4096,
        argon2_t_cost: 1,
        argon2_p_cost: 2,
      },
    }
  }
}

fn default_user_check() {
  if !usersys::UserData::<()>::check_users_exist(
    &crate::CONFIG.maintenance_page.usersys_config,
  )
  .unwrap()
  {
    let default_user = usersys::UserData::new(
      &crate::CONFIG.maintenance_page.initial_username,
      &crate::CONFIG.maintenance_page.initial_pswd,
      None::<()>,
      &crate::CONFIG.maintenance_page.usersys_config,
    )
    .unwrap();
    default_user
      .save(&crate::CONFIG.maintenance_page.usersys_config)
      .unwrap();
  }
}

pub(crate) fn mainte_serve() -> Router {
  default_user_check();
  Router::new().route("/", post(mainte_page_main))
}

#[derive(Debug, Deserialize, Serialize)]
struct MaintePageForm {
  #[serde(flatten)]
  main: crate::main_page::MainArgs,
  #[serde(alias = "admin-name")]
  admin_name: String,
  #[serde(alias = "admin-password")]
  admin_password: String,
  #[serde(alias = "new-username")]
  new_username: Option<String>,
  #[serde(alias = "new-password")]
  new_password: Option<String>,
  #[serde(alias = "new-password-verify")]
  new_password_verify: Option<String>,
}

enum ChangeUserDataMode<'a> {
  NewUser {
    new_username: &'a str,
    new_password: &'a str,
  },
  PswdChange {
    new_password: &'a str,
  },
  PswdIsTooShort,
  PswdInvalid,
  PswdEmptyNotAllow,
  UserNameDuplicate,
  Nop,
}
impl<'a> From<&'a MaintePageForm> for ChangeUserDataMode<'a> {
  fn from(form: &'a MaintePageForm) -> Self {
    match (
      form
        .new_password
        .as_ref()
        .map(|s| s.trim())
        .filter(|np| !np.is_empty()),
      form
        .new_password_verify
        .as_ref()
        .map(|s| s.trim())
        .filter(|np| !np.is_empty()),
      form
        .new_username
        .as_ref()
        .map(|s| s.trim())
        .filter(|np| !np.is_empty()),
    ) {
      (
        Some(new_password),
        Some(new_password_verify),
        new_username,
      ) if new_password == new_password_verify => {
        let new_username = match new_username {
          Some(new_username) => {
            if new_username == form.admin_name.as_str() {
              None
            } else {
              if crate::usersys::UserData::<()>::check_exist(
                new_username,
                &crate::CONFIG.maintenance_page.usersys_config,
              )
              .unwrap()
              {
                return Self::UserNameDuplicate;
              }
              Some(new_username)
            }
          }
          None => None,
        };
        if crate::CONFIG.maintenance_page.pswd_len_min
          <= new_password.len()
        {
          match new_username {
            Some(new_username) => Self::NewUser {
              new_username,
              new_password,
            },
            None => Self::PswdChange { new_password },
          }
        } else {
          Self::PswdIsTooShort
        }
      }
      (None, None, Some(_)) => Self::PswdEmptyNotAllow,
      (None, None, None) => Self::Nop,
      _ => Self::PswdInvalid,
    }
  }
}

async fn mainte_page_main(
  Form(mainte): Form<MaintePageForm>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  let ch_ud_mode = ChangeUserDataMode::from(&mainte);
  let user_data = match usersys::UserData::<()>::load(
    &mainte.admin_name,
    &mainte.admin_password,
    match ch_ud_mode {
      ChangeUserDataMode::PswdChange { new_password } => {
        Some(new_password)
      }
      _ => None,
    },
    &crate::CONFIG.maintenance_page.usersys_config,
    || Ok(()),
  ) {
    Ok(ud) => ud,
    Err(e) => {
      return Err(crate::bsod::bsod(
        StatusCode::BAD_REQUEST,
        None,
        None,
      ));
    }
  };
  let Some(mut user_data) = user_data else {
    return Err(crate::bsod::bsod(
      StatusCode::FORBIDDEN,
      None,
      None,
    ));
  };

  let mut output = String::new();
  page_gen::page_gen(
    &mut output,
    &mut user_data,
    ch_ud_mode,
    &mainte,
  )
  .unwrap();

  Ok(Html(output))
}

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

#[derive(Deserialize, Serialize)]
pub struct MaintePageConfig {
  pub password_dir: String,
  pub initial_username: String,
  pub initial_pswd: String,
  pub usersys_config: usersys::UserDataConfig,
}
impl Default for MaintePageConfig {
  fn default() -> Self {
    Self {
      password_dir: "mainte-pswd".into(),
      initial_username: "Admin01".into(),
      initial_pswd: "D3fau1tPassw0rd".into(),
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
  if !usersys::UserData::<()>::check_exist(
    &crate::CONFIG.maintenance_page.initial_username,
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
}

async fn mainte_page_main(
  Form(mainte): Form<MaintePageForm>,
) -> Result<impl IntoResponse, impl IntoResponse> {
  let user_data = match usersys::UserData::<()>::load(
    &mainte.admin_name,
    &mainte.admin_password,
    &crate::CONFIG.maintenance_page.usersys_config,
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
  if let Some(user_data) = user_data {
    Ok(Html("OK"))
  } else {
    Err(crate::bsod::bsod(StatusCode::FORBIDDEN, None, None))
  }
}

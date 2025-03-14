use std::{
  fs::File, io::BufWriter, net::Ipv4Addr, sync::LazyLock,
};

pub mod main_page;
pub mod mainte;
pub mod service;
pub mod usersys;

use axum::{Router, routing::get};
use chrono::{DateTime, Utc};
use env_logger::Target;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

pub const COMMON_CSS: &'static str =
  include_str!("styles/common.css");
pub const MAIN_CSS: &'static str =
  include_str!("styles/main.css");

#[derive(Deserialize, Serialize)]
pub struct Config {
  pub origin_time: DateTime<Utc>,
  pub listen_port: u16,
  pub log_file: String,
  pub maintenance_page: mainte::MaintePageConfig,
  pub service: service::ServiceConfig,
}
impl Default for Config {
  fn default() -> Self {
    Self {
      origin_time: DateTime::parse_from_rfc3339(
        "2025-01-01T00:00:00.00Z",
      )
      .unwrap()
      .into(),
      listen_port: 8080,
      log_file: "tmdx4-workplace.log".into(),
      maintenance_page: mainte::MaintePageConfig::default(),
      service: service::ServiceConfig::default(),
    }
  }
}

const CONFIG_FILENAME: &'static str = "config.json";
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
  let fp = match std::fs::File::open(CONFIG_FILENAME) {
    Ok(fp) => std::io::BufReader::new(fp),
    Err(e) => match e.kind() {
      std::io::ErrorKind::NotFound => {
        let mut fp = std::io::BufWriter::new(
          std::fs::File::create(CONFIG_FILENAME).unwrap(),
        );
        eprintln!(
          "コンフィグファイルが存在しない為、初期設定ファイルを生成します"
        );
        <BufWriter<std::fs::File> as std::io::Write>::write(
          &mut fp,
          &serde_json::to_vec_pretty(&Config::default())
            .unwrap(),
        )
        .unwrap();
        eprintln!(
          "内容を適宜なものとして、再度アプリケーションの起動をお願いします。"
        );
        panic!();
      }
      _ => panic!("{e}"),
    },
  };
  serde_json::from_reader(fp).unwrap()
});

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  env_logger::builder()
    .filter_level(if cfg!(debug_assertions) {
      log::LevelFilter::Debug
    } else {
      log::LevelFilter::Info
    })
    .target(Target::Pipe({
      let fp = File::create("./tmdx4-workplace.log")?;
      Box::from(BufWriter::new(fp))
    }))
    .build();
  let app = Router::new()
    .route("/", get(move |q| main_page::main_page(q)))
    .nest("/mainte", mainte::mainte_serve());
  let listener = TcpListener::bind((
    Ipv4Addr::from([0, 0, 0, 0]),
    CONFIG.listen_port,
  ))
  .await?;
  axum::serve(listener, app).await?;
  Ok(())
}

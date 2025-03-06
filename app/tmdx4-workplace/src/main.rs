use std::{
  fmt::Write,
  fs::File,
  io::{BufWriter, Read},
  net::Ipv4Addr,
  sync::{
    Arc, LazyLock, OnceLock,
    atomic::{AtomicU64, Ordering},
  },
};

pub mod mainte;
pub mod service;

use axum::{
  Form, Router,
  extract::{Query, RawQuery},
  response::{Html, IntoResponse},
  routing::{get, post},
};
use chrono::{DateTime, Utc};
use env_logger::Target;
use parking_lot::{RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

pub const COMMON_CSS: &'static str = include_str!("styles/common.css");
pub const MAIN_CSS: &'static str = include_str!("styles/main.css");
pub const SIDE_CSS: &'static str = include_str!("styles/side.css");

#[derive(Deserialize, Serialize)]
pub struct MaintePageConfig {
  pub password_file: String,
  pub initial_pswd: String,
  pub argon2_mem_cost: u32,
  pub argon2_time_cost: u32,
  pub argon2_par_cost: u32,
}
impl Default for MaintePageConfig {
  fn default() -> Self {
    Self {
      password_file: "mainte-pswd.bin".into(),
      initial_pswd: "D3fau1tPassw0rd".into(),
      argon2_mem_cost: 4096,
      argon2_time_cost: 1,
      argon2_par_cost: 2,
    }
  }
}

#[derive(Deserialize, Serialize)]
pub struct Config {
  pub origin_time: DateTime<Utc>,
  pub listen_port: u16,
  pub log_file: String,
  pub maintenance_page: MaintePageConfig,
  pub service: service::ServiceConfig,
}
impl Default for Config {
  fn default() -> Self {
    Self {
      origin_time: DateTime::parse_from_rfc3339("2025-01-01T00:00:00.00Z")
        .unwrap()
        .into(),
      listen_port: 8080,
      log_file: "tmdx4-workplace.log".into(),
      maintenance_page: MaintePageConfig::default(),
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
        let mut fp = std::io::BufWriter::new(std::fs::File::create(CONFIG_FILENAME).unwrap());
        eprintln!("コンフィグファイルが存在しない為、初期設定ファイルを生成します");
        <BufWriter<std::fs::File> as std::io::Write>::write(
          &mut fp,
          &serde_json::to_vec_pretty(&Config::default()).unwrap(),
        )
        .unwrap();
        eprintln!("内容を適宜なものとして、再度アプリケーションの起動をお願いします。");
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

  let c = SimpleTinyCounter::new(0);
  let articles1 = Articles(Arc::new(RwLock::new(Vec::new())));
  let articles2 = articles1.clone();
  let app = Router::new()
    .route(
      "/",
      get(move |q| main_page(q, c.clone(), articles1.clone())),
    )
    .nest("/mainte", mainte::mainte_serve(articles2));
  let listener = TcpListener::bind((Ipv4Addr::from([0, 0, 0, 0]), CONFIG.listen_port)).await?;
  axum::serve(listener, app).await?;
  Ok(())
}

struct SimpleTinyCounter(Arc<AtomicU64>);
impl SimpleTinyCounter {
  pub fn new(v: u64) -> Self {
    Self(Arc::new(AtomicU64::new(v)))
  }
  pub fn countup(&self) -> u64 {
    let v = self.0.load(Ordering::Acquire) + 1;
    self.0.store(v, Ordering::Release);
    v
  }
}
impl Clone for SimpleTinyCounter {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

struct Article {
  title: String,
  body: String,
  update: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
struct Articles(Arc<RwLock<Vec<Article>>>);

async fn main_page(q: RawQuery, c: SimpleTinyCounter, articles: Articles) -> impl IntoResponse {
  let c = c.countup();
  let viewmode = q.0.map(|q| {
    let mut nightmode = false;
    let mut showside = false;
    let mut sideinverse = false;
    let mut noframe = false;
    let mut maximize = false;
    q.split('&')
      .filter_map(|s| s.split_once('='))
      .map(|s| s.0)
      .for_each(|s| match s {
        "nightmode" => nightmode = true,
        "showside" => showside = true,
        "sideinverse" => sideinverse = true,
        "noframe" => noframe = true,
        "maximize" => maximize = true,
        _ => {}
      });
    (nightmode, showside, sideinverse, noframe, maximize)
  });
  let mut buffer = format!("\
    <!doctype html>\
    <html lang=\"ja\">\
      <head>\
        <meta charset=\"utf-8\">\
        <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
        <meta name=\"format-detection\" content=\"telephone=no,email=no,address=no\">\
        <title>ツナマヨの屋根裏部屋</title>\
        <link rel=\"icon\" href=\"assets/img/com/favicon.webp\">\
        <meta name=\"description\" content=\"しがない創作者ツナ・マヨネーズの作業部屋\">\
        <style>\
          {COMMON_CSS}\
          {MAIN_CSS}\
          {SIDE_CSS}\
        </style>\
      </head>\
      <body>\
        <form method=\"GET\" action=\"?\" id=\"content-area\">\
          <header>\
            <section id=\"head-bar\">\
              <img src=\"assets/img/com/favicon-mini.webp\" width=\"16\" height=\"16\" alt=\"\">\
              <div id=\"head-bar-title\"><h1>ツナマヨの屋根裏部屋</h1></div>\
              <div id=\"head-bar-button\">\
                <label for=\"nightmode\"><input type=\"checkbox\" name=\"nightmode\" id=\"nightmode\" value=\"nightmode\" {nightmode}>★</label>\
                <div class=\"view-mob-only\">\
                  <label for=\"showside\"><input type=\"checkbox\" name=\"showside\" id=\"showside\" value=\"showside\" {showside}>＜</label>\
                </div>\
                <div class=\"view-pc-only\">\
                  <label for=\"sideinverse\"><input type=\"checkbox\" name=\"sideinverse\" id=\"sideinverse\" value=\"sideinverse\" {sideinverse}>◇</label>\
                  <label for=\"noframe\"><input type=\"checkbox\" name=\"noframe\" id=\"noframe\" value=\"noframe\" {noframe}>＊</label>\
                  <label for=\"maximize\"><input type=\"checkbox\" name=\"maximize\" id=\"maximize\" value=\"maximize\" {maximize}>□</label>\
                </div>
              </div>\
            </section>\
            <section id=\"menu-bar\">\
            </section>\
          </header>\
          <div id=\"noframe-scroll\">\
            <main>\
              <article id=\"main\">\
                <header>\
                  <div class=\"page-title\" id=\"main-content-title-area\"><div><div><h2>ツナマヨの屋根裏部屋</h2></div></div></div>\
                  <span class=\"rainbow\">★</span>あなたは{c}人目のお客様です！<span class=\"rainbow\">★</span><br>\
                  <section id=\"notice\">\
                    <div>お知らせはございません。</div>\
                  </section>\
                </header>\
                <main>\
    ",
    nightmode = viewmode.map_or("", |v| if v.0 {"checked"} else { "" }),
    showside = viewmode.map_or("", |v| if v.1 {"checked"} else { "" }),
    sideinverse = viewmode.map_or("", |v| if v.2 {"checked"} else { "" }),
    noframe = viewmode.map_or("", |v| if v.3 {"checked"} else { "" }),
    maximize = viewmode.map_or("", |v| if v.4 {"checked"} else { "" }),
  );

  for article in articles.0.read().iter() {
    buffer
      .write_fmt(format_args!(
        "\
        <article class=\"dialy\">\
          <header>\
            <h3>{title}</h3>\
            <span id=\"dialy-update\">{update}</span>\
          </header>\
          <main>\
            {body}\
          </main>\
          <footer>\
          </footer>\
        </article>\
      ",
        title = article.title,
        update = article.update.format("%Y-%m-%d %H:%M"),
        body = article.body,
      ))
      .unwrap();
  }

  buffer += "\
                </main>\
              </article>\
              <article id=\"side\"><div class=\"sys-style-marker\">\
                <header>\
                  <label for=\"sidebar-goto-top\" style=\"text-decoration: none;\" class=\"page-title\" id=\"side-content-title-area\">
                    <input id=\"sidebar-goto-top\" type=\"submit\" formaction=\"/\">\
                    <div><div>ツナマヨの屋根裏部屋</div></div>\
                  </label>\
                </header>\
                <main>\
                </main>\
                <footer>\
                  このページはMozilla Firefox 135, Microsoft Edge 133, Google Chrome 133にてテストされております。<br>\
                  適切に表示することを望まれる場合には、2024年以降にリリースされたバージョンのブラウザをご使用ください。<br>\
                  <hr>\
                  連絡先：https://misskey.design/@tunatuna486<br>\
                  <div id=\"login-admin\">\
                    <input type=\"password\" name=\"password\" width=\"24\" formaction=\"mainte\">\
                    <input type=\"submit\" value=\"管理画面へログイン\" formaction=\"mainte\" formmethod=\"POST\">\
                  </div>\
                </footer>\
              </div></article>\
            </main>\
            <footer>\
              <div id=\"sign\">2025 This page written by TunamayoDX4</div>\
            </footer>\
          </div>\
        </form>\
      </body>\
    </html>\
  ";
  Html(buffer)
}

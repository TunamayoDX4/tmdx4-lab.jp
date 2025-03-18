//! メインページの各種実装

use serde::{Serialize, Deserialize};
use axum::{
  extract::Query, 
  response::{IntoResponse, Html}, 
};
use crate::{
  COMMON_CSS, 
  MAIN_CSS, 
};
pub mod frame;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ViewMode {
  #[default]
  #[serde(alias = "daytime")]
  DayTime,
  #[serde(alias = "night")]
  Night,
}
impl std::fmt::Display for ViewMode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(match self {
      ViewMode::DayTime => "daytime",
      ViewMode::Night => "night",
    })
  }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct IsSelected(bool);
pub struct IsSelectedVisitor;
impl<'de> serde::de::Visitor<'de> for IsSelectedVisitor {
  type Value = IsSelected;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("struct IsSelected")
  }

  fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(match v.trim() {
      "on" => IsSelected(true),
      _ => IsSelected(false),
    })
  }
}
impl<'de> serde::Deserialize<'de> for IsSelected {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_str(IsSelectedVisitor)
  }
}
impl Default for IsSelected {
  fn default() -> Self {
    Self(false)
  }
}
impl std::fmt::Display for IsSelected {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.0 {
      f.write_str("checked")?
    }
    Ok(())
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MainArgs {
  #[serde(alias = "view-mode", default)]
  pub view_mode: ViewMode,
  #[serde(default)]
  pub maximize: IsSelected,
  #[serde(default)]
  pub noheader: IsSelected,
  #[serde(default)]
  pub notaskbar: IsSelected,
  #[serde(default)]
  pub invframe: IsSelected,
  #[serde(default)]
  pub noframe: IsSelected,
}

pub async fn main_page(Query(mq): Query<MainArgs>) -> impl IntoResponse {
  let buffer = format!(
    "\
      <!doctype html>\
      <html lang='ja'>\
        <head>\
          <meta charset='utf-8'>\
          <meta name='viewport' content='width=device-width,initial-scale=1,minimum-scale=1'>\
          <meta name='format-detection' content='telephone=no,email=no,address=no'>\
          <title>ツナマヨの屋根裏部屋</title>\
          <link rel='icon' href='assets/img/com/favicon.webp'>\
          <meta name='description' content='しがない創作者ツナ・マヨネーズの作業部屋。趣味で作ったイラストやプログラム、漫画などを公開していきます。'>\
          <link rel='stylesheet' href='https://fonts.googleapis.com/css2?family=Mochiy+Pop+One'>\
          <style>{COMMON_CSS}</style>\
          <style>{MAIN_CSS}</style>\
        </head>\
        <body>\
          <form action='' method='GET' id='trans-ownpage'></form>
          <section id='main-area'>
            <article class='ui-window' id='main-window'>
              <header>
                <section class='window-header-line'>
                  <div class='window-hl-left'>
                    <img class='window-title-icon' src='./assets/img/com/favicon-mini.webp' alt=''>
                    <h1 class='window-title'>ツナマヨの屋根裏部屋</h1>
                  </div>
                  <div class='window-hl-right'>
                    <div class='ctx-button'>
                      <fieldset>
                        <label for='daytime' class='common-button'>
                          <input form='trans-ownpage' type='radio' name='view-mode' id='daytime' value='daytime' {mode_daytime}>☀
                        </label>
                        <label for='night' class='common-button'>
                          <input form='trans-ownpage' type='radio' name='view-mode' id='night' value='night' {mode_night}>☾
                        </label>
                      </fieldset>
                      <hr>
                      <div class='button-array'>
                        <label for='minimize' class='common-button hidden-checked-active pc-only'>
                          <input form='trans-ownpage' type='checkbox' name='minimize' id='minimize'>－
                        </label>
                        <label for='maximize' class='common-button hidden-checked-active pc-only'>
                          <input form='trans-ownpage' type='checkbox' name='maximize' id='maximize' {mq_maximize}>
                          <span class='with-disable'>□</span>
                          <span class='with-enable'>
                            <span style='font-size: 0.7em'>□</span>
                          </span>
                        </label>
                        <div class='common-button pc-only'>×</div>
                      </div>
                      <div class='button-array'>
                        <label for='toggleframe' class='common-button hidden-checked-active mob-only'>
                          <input form='trans-ownpage' type='checkbox' name='toggleframe' id='toggleframe'>
                          <span class='with-disable'>＜</span>
                          <span class='with-enable'>＞</span>
                        </label>
                      </div>
                    </div>
                  </div>
                </section>
                <section class='window-header-menu'>
                  <section class='window-header-pulldown-list row-ui' style='z-index: 1000;'>
                    <hr class='sep-thick'>
                    <nav class='common-button common-pulldown flat-type' id='menu-navi' style='--border-thickness: 1px'>
                      ﾅﾋﾞｹﾞｰｼｮﾝ(N)
                      <ul>
                        <li class='common-button flat-type' style='--border-thickness: 1px'>あああ</li>
                        <li class='common-button flat-type' style='--border-thickness: 1px'>いいい</li>
                      </ul>
                    </nav>
                    <div class='common-button common-pulldown flat-type' id='menu-favorite' style='--border-thickness: 1px'>
                      お気に入り(F)
                      <ul>
                        <li class='common-button flat-type' style='--border-thickness: 1px'>管理人のMissKey Design</li>
                        <li class='common-button flat-type' style='--border-thickness: 1px'>かかか</li>
                        <li class='common-button flat-type' style='--border-thickness: 1px'>ききき</li>
                        <li class='common-button flat-type' style='--border-thickness: 1px'>くくく</li>
                      </ul>
                    </div>
                    <div class='common-button common-pulldown flat-type' id='menu-view' style='--border-thickness: 1px'>
                      表示(V)
                      <ul>
                        <li><label class='common-button flat-type' for='maximize' style='--border-thickness: 1px'>ｳｨﾝﾄﾞｳの最大化</label></li>
                        <li><label class='common-button flat-type' for='noheader' style='--border-thickness: 1px'>ｳｨﾝﾄﾞｳﾍｯﾀﾞｰの非表示<input type='checkbox' id='noheader' name='noheader' form='trans-ownpage' {mq_noheader}></label></li>
                        <li><label class='common-button flat-type' for='notaskbar' style='--border-thickness: 1px'>タスクバーの非表示<input type='checkbox' id='notaskbar' name='notaskbar' form='trans-ownpage' {mq_notaskbar}></label></li>
                        <li><label class='common-button flat-type' for='noframe' style='--border-thickness: 1px'>フレームの非表示<input type='checkbox' id='noframe' name='noframe' form='trans-ownpage' {mq_noframe}></label></li>
                        <li><label class='common-button flat-type' for='invframe' style='--border-thickness: 1px'>ﾌﾚｰﾑ位置の左右反転<input type='checkbox' id='invframe' name='invframe' form='trans-ownpage' {mq_invframe}></label></li>
                        <li><label class='common-button flat-type' for='daytime' style='--border-thickness: 1px'>昼間モード</label></li>
                        <li><label class='common-button flat-type' for='night' style='--border-thickness: 1px'>夜間モード</label></li>
                      </ul>
                    </div>
                    <div class='common-button common-pulldown flat-type' id='menu-help' style='--border-thickness: 1px'>
                      ヘルプ(H)
                      <ul>
                        <li class='common-button flat-type' style='--border-thickness: 1px'>マニュアル</li>
                        <li class='common-button flat-type' style='--border-thickness: 1px'>ツナマヨの屋根裏部屋について</li>
                      </ul>
                    </div>
                  </section>
                  <section class='window-header-right-logo'>
                    <img class='window-header-right-logo-icon' src='./assets/img/com/favicon-mini.webp' alt=''>
                  </section>
                </section>
                <section class='window-header-menu'  style='z-index: 900;'>
                  <section class='row-ui'>
                    <hr class='sep-thick'>
                    <label for='search-on-page' class='common-button flat-type' style='--border-thickness: 1px'>
                      <input type='submit' id='search-on-page' form='trans-ownpage' formaction='search' formmethod='get'>
                      ﾍﾟｰｼﾞ内検索(S)
                    </label>
                    <input class='common-text-input' type='text' name='search-string' form='trans-ownpage' style='margin-inline: 0.5rem; flex: 1;'>
                  </section>
                </section>
              </header>
              <main>
                <article class='window-graphic-obj' id='side-frame'>
                  <header>
                    <label class='funny-logo'>
                      <input type='submit' form='trans-ownpage' style='display: none;'>
                      <div><div>ツナマヨの屋根裏部屋</div></div>
                    </label>
                  </header>
                  <hr>
                  <main></main>
                  <hr>
                  <footer>
                    このページは<br>Mozilla Firefox 136<br>Google Chrome 133<br>Microsoft Edge 133<br>
                    にてテストをしております。<br>
                    <hr>
                    ページレイアウトを適切に表示するためには、<br>
                    お手数ですが2024年以降にリリースされたバージョンのブラウザでのアクセスをお願いします。
                    <hr>
                    <img src='assets/img/banner/banner01.png' alt='バナー01'>
                    <hr>
                    <div id='admin-only'>
                      <label class='common-button' for='enter-adm-window-open'>
                        <input type='checkbox' id='enter-adm-window-open'>
                        管理用ﾍﾟｰｼﾞへのﾛｸﾞｲﾝ
                      </label>
                    </div>
                  </footer>
                </article>
                <article class='window-graphic-obj' id='main-content'>
                  <header>
                    <div class='funny-logo'>
                      <div><div><h2>ツナマヨの屋根裏部屋</h2></div></div>
                    </div>
                    <section id='counter'>
                      <div id='daily'><span class='rainbow'>★</span>あなたは××××人目のお客様です！<span class='rainbow'>★</span></div>
                    </section>
                    <section class='marquee'><div>お知らせはございません。</div></section>
                  </header>
                  <hr>
                  <main>
                    <div class='debug' style='white-space: pre-line;'>
                      {mq:?}
                    </div>
                  </main>
                  <hr>
                  <footer>
                  </footer>
                </article>
              </main>
              <footer>
                <span id='sign'>2025 This page written by TunamayoDX4</span>
              </footer>
            </article>
            <section id='enter-adm-window' class='ui-window'>
              <header>
                <section class='window-header-line'>
                  <div class='window-hl-left window-title'>管理ﾍﾟｰｼﾞのﾛｸﾞｲﾝ</div>
                  <div class='window-hl-right ctx-button'>
                    <label for='enter-adm-window-open' class='common-button'>×</label>
                  </div>
                </section>
              </header>
              <main style='display: flex; flex-flow: column;'>
                <div style='display: flex; flex-flow: row;'>
                  <label for='input-admin-name' style='width: 10rem;'>管理ﾕｰｻﾞ名</label>：
                  <input class='common-text-input' type='text' name='admin-name' form='trans-ownpage' style='width: 100%;' id='input-admin-name'>
                </div>
                <div style='display: flex; flex-flow: row;'>
                  <label for='input-admin-pswd' style='width: 10rem;'>パスワード</label>：
                  <input class='common-text-input' type='password' name='admin-password' form='trans-ownpage' style='width: 100%' id='input-admin-pswd'>
                </div>
                <div style='width: 100%; display: flex; flex-flow: row; align-items: center; align-content: center; justify-content: space-between; padding-inline: 1rem; margin: 0.25rem;'>
                  <label for='enter-admin' class='common-button' style='padding-inline: 0.5rem;'>
                    <input type='submit' id='enter-admin' form='trans-ownpage' formaction='mainte' formmethod='post'>ﾛｸﾞｲﾝ
                  </label>
                  <label for='enter-adm-window-open' class='common-button hidden-checked-active' style='padding-inline: 0.5rem;'>ｷｬﾝｾﾙ</label>
                </div>
              </main>
            </section>
          </section>
          <article class='ui-bar' id='taskbar'>
            <header class='row-ui'>
              <label class='common-button' for='start-ctx-button' style='font-weight: bolder;'>
                <input type='checkbox' id='start-ctx-button'>
                ｽﾀｰﾄ
              </label>
              <hr class='sep-thin'>
              <hr class='sep-thick'>
            </header>
            <main>
            </main>
            <footer class='row-ui'>
            </footer>
          </article>
          <label for='start-ctx-button' class='cb-closure'></label>
          <label for='enter-adm-window-open' class='cb-closure'></label>
          <style>
            body:has(article#taskbar > header input#start-ctx-button:checked) > label.cb-closure[for='start-ctx-button'] {{
              display: block;
            }}
            body:has(section#main-area > article#main-window > main > #side-frame input#enter-adm-window-open:checked) > label.cb-closure[for='enter-adm-window-open'] {{
              display: block;
            }}
          </style>
        </body>\
      </html>\
    ",
    mode_daytime = match mq.view_mode {
      ViewMode::DayTime => "checked", 
      _ => "", 
    }, 
    mode_night = match mq.view_mode {
      ViewMode::Night => "checked", 
      _ => "", 
    }, 
    mq_maximize = mq.maximize, 
    mq_noheader = mq.noheader, 
    mq_notaskbar = mq.notaskbar,
    mq_noframe = mq.noframe, 
    mq_invframe = mq.invframe, 
  );
  Html(buffer)
}

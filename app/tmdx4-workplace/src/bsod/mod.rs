use std::borrow::Cow;

use axum::{
  http::StatusCode,
  response::{Html, IntoResponse},
};

const BSOD_STRING: &'static [&'static [Option<(
  &'static str,
  &'static [&'static str],
  Option<&'static str>,
)>]] = &[
  &[],
  &[],
  &[],
  &[],
  &[
    Some((
      "400 BAD REQUEST",
      &[
        "要求が不正の為処理が不能・もしくは実行に不適の為、正常なリクエストの生成が出来ませんでした。",
        "TIPS: クエリパラメータ・フォーム内容をご確認ください。",
      ],
      None,
    )),
    None,
    None,
    Some((
      "403 FORBIDDEN",
      &[
        "クライアントはサーバの該当コンテンツへのアクセス権がありません。",
        "TIPS: URIを確認してください。",
      ],
      None,
    )),
    Some((
      "404 NOT FOUND",
      &[
        "要求されたリクエストはURIが誤っているか、リソース自体がサーバに存在しておりません。",
        "TIPS: URIを確認し、スペルミスおよびワードチョイスや数値のミスが無いかをご確認ください。",
      ],
      None,
    )),
  ],
  &[],
];
const BSOD_DEFAULT: [usize; 2] = [4, 3];
const BSOD_DEFAULT_TODO: &'static str =
  "任意のｱﾄﾞﾚｽを入力するか、前のページにお戻りください.";

enum BsodString {
  String(Cow<'static, str>),
  Array(&'static [&'static str]),
}
impl std::fmt::Display for BsodString {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
    match self {
      Self::String(s) => f.write_str(&s)?,
      Self::Array(a) => {
        for s in a.iter() {
          f.write_str(s)?;
          f.write_str("<br>")?;
        }
      }
    }
    Ok(())
  }
}

pub fn bsod(
  status_code: StatusCode,
  error_msg: Option<Cow<'static, str>>,
  todo_msg: Option<Cow<'static, str>>,
) -> impl IntoResponse {
  let ss = status_code.as_str().chars();
  let (head, tail) = (
    ss.clone().nth(0).unwrap(),
    [ss.clone().nth(1).unwrap(), ss.clone().nth(2).unwrap()],
  );
  let head = head.to_digit(10).unwrap();
  let tail = tail[0].to_digit(10).unwrap() * 10
    + tail[1].to_digit(10).unwrap();
  let head = head as usize;
  let tail = tail as usize;
  let (error_code, text, todo) =
    BSOD_STRING.get(head).map(|s| s[tail]).flatten().unwrap_or(
      BSOD_STRING[BSOD_DEFAULT[0]][BSOD_DEFAULT[1]].unwrap(),
    );
  let todo = error_msg
    .unwrap_or(todo.unwrap_or(BSOD_DEFAULT_TODO).into());
  let text = todo_msg
    .map(|v| BsodString::String(v))
    .unwrap_or(BsodString::Array(text));
  (
    status_code,
    Html(format!(
      "<!doctype html>
      <html lang='ja'>
        <head>
          <meta charset='utf-8'>
          <meta name='viewport' content='width=device-width,initial-scale=1,minimum-scale=1'>
          <title>404 NOT FOUND</title>
          <style>
            * {{
              margin: 0;
              padding: 0;
              border: none;
              box-sizing: border-box;
              color: white;
              white-space: wrap;
              overflow-wrap: break-word;
              word-break: break-all;
              font-family: 'MS UI Gothic';
            }}

            html {{
              width: 100%;
              height: 100%;
              background-color: darkblue;

              display: flex;
              flex-flow: column;
              justify-content: center;
              align-items: center;

              > body {{
                min-width: 18rem;
                max-width: 96rem;
                display: flex;
                flex-flow: column;
                justify-content: center;
                align-items: center;

                > div#title {{
                  width: fit-content;
                  padding-inline: 0.5rem;
                  background-color: lightgray;
                  > h1 {{
                    font-size: 1rem;
                    font-weight: bolder;
                    color: darkblue;
                  }}
                }}

                > div#message {{
                  margin-top: 1rem;
                  width: 100%;

                  > div.align-right {{
                    width: 100%;
                    text-align: right;
                  
                  }}
                }}
              }}
            }}
          </style>
        </head>
        <body>
          <div id='title'><h1>{error_code}</h1></div>
          <div id='message'>
            {text}
            <br>
            <div class='align-right'>{todo}</div>
          </div>
        </body>
      </html>
      ", 
    )),
  )
}

use std::borrow::Cow;

use crate::usersys::{self, UserData};

pub(super) fn page_gen(
  write: &mut impl std::fmt::Write,
  user_data: &mut crate::usersys::UserData<()>,
  ch_ud_mode: super::ChangeUserDataMode,
  form: &super::MaintePageForm,
) -> Result<(), Box<dyn std::error::Error>> {
  write.write_fmt(format_args!("\
      <!doctype html>
      <html lang='ja'>
        <head>
          <meta charset='utf-8'>
          <title>メンテナンスページ</title>
        </head>
        <body>
          <form action='' method='POST' id='trans-ownpage'></form>
          <h1>メンテナンスページ</h1>
          {change_pswd_msg}
          <hr>
          <h2>管理ユーザの設定</h2>
          ユーザ識別子: {ident}
          <input type='hidden' name='admin-name' value='{username}'>
          <input type='hidden' name='admin-password' value='{password}'>
          <input type='text' name='new-username' form='trans-ownpage'>
          <input type='password' name='new-password' form='trans-ownpage'>
          <input type='password' name='new-password-verify' form='trans-ownpage'>
          <input type='submit' name='submit' value='送信' form='trans-ownpage'>
          <hr>
          <form action='upload_article' method='POST' id='upload_article'>
            <input type='text' name='title'>
            <input type='textarea' name='text'>
            <input type='submit' name='submit' value='送信'>
          </form>
        </body>
      </html>
    ",
    change_pswd_msg = match ch_ud_mode{
      super::ChangeUserDataMode::NewUser { new_username, new_password } => {
        UserData::new(new_username, new_password, (), &crate::CONFIG.maintenance_page.usersys_config).unwrap().save(&crate::CONFIG.maintenance_page.usersys_config).unwrap();
        Cow::from("新しいユーザの登録")
      },
      super::ChangeUserDataMode::PswdChange { new_password: _ } => {
        Cow::from("パスワードの変更")
      },
      super::ChangeUserDataMode::PswdIsTooShort => Cow::from(format!("パスワードの長さは{}以上にしてください", crate::CONFIG.maintenance_page.pswd_len_min)),
      super::ChangeUserDataMode::PswdInvalid => Cow::from("新旧のパスワードが一致しません"),
      super::ChangeUserDataMode::PswdEmptyNotAllow => Cow::from("ユーザ登録時にはパスワードを入力してください"),
      super::ChangeUserDataMode::UserNameDuplicate => Cow::from("ユーザ名が重複しています"),
      super::ChangeUserDataMode::Nop => Cow::from(""),
    },
      username = form.admin_name.as_str(),
      password = match ch_ud_mode {
        super::ChangeUserDataMode::PswdChange { new_password } => new_password, 
        _ => form.admin_password.as_str(), 
      },
      ident = user_data.ident(),
    ))?;
  Ok(())
}

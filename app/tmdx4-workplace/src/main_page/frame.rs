//! メインページのフレーム生成プログラム

use std::fmt::{Display, Write};

struct Frame<H, M, F>
where
  H: Display,
  M: Display,
  F: Display,
{
  header: H,
  main: M,
  footer: F,
}

pub fn gen_frame(wrt: &mut impl Write) {}

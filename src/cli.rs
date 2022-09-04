use clap::{Parser};

use crate::render::ToneType;

/// 在命令行中练习双拼
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
   /// 如何显示拼音
   #[clap(short, long, value_parser, default_value = "live", value_name = "TYPE")]
   pub  pinyin: ToneType,

   /// 键位路径
   #[clap(short, long, value_name = "DIR", value_hint = clap::ValueHint::DirPath, value_parser, default_value = "./keymap/ms")]
   pub keymap: String,

   /// 文本路径
   #[clap(short, long, value_name = "DIR", value_hint = clap::ValueHint::DirPath, value_parser, default_value = "./text/text.txt")]
   pub text: String,
}

impl clap::ValueEnum for ToneType {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Always, Self::Live, Self::Off]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::PossibleValue<'a>> {
        match self {
            Self::Always => Some(clap::PossibleValue::new("always")),
            Self::Live => Some(clap::PossibleValue::new("live")),
            Self::Off => Some(clap::PossibleValue::new("off")),
        }
    }
}
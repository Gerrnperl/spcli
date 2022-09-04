use std::{fs, io, cmp, fmt::Write};
use crossterm::{Command, style::{PrintStyledContent, Stylize, Color}};
use rand::{self, Rng};
use pinyin::{ToPinyin, Pinyin};
use unicode_width::UnicodeWidthChar;
use core::fmt::Debug;

#[derive(Debug)]
pub struct Document {
	pub passages: Vec<Passage>,
}


impl Document {
	pub fn open(filepath: &str) -> Result<Self, io::Error> {
		let file = fs::read_to_string(filepath).unwrap();
		let passages: Vec<Passage> 
		= file.split("\n\n").map(|passage| {
			Passage::new(passage.to_string())
		}).collect();
		Ok(Self {
			passages,
		})
	}

	pub fn get_random(&self) -> usize {
		rand::thread_rng().gen_range(0..self.passages.len())
	}
}


pub struct Passage {
	pub chars: Vec<Character>,
}

impl Passage {
	pub fn new(string: String) -> Self {
		let string = " ".to_string() + &string;
		let pinyin = Self::get_pinyin(string.clone());
		let mut pinyin_iter = pinyin.into_iter();
		let mut chars = Vec::new();
		string.chars().for_each(|c| {
			if let Some(p) = pinyin_iter.next() {
				chars.push(
					Character::new(c, p)
				);
			}
		});
		Self {
			chars,
		}
	}
	pub fn get_pinyin(string: String) -> Vec<Option<Pin>> {
		let pinyin_raw = string.as_str().to_pinyin();
		let mut result = Vec::new();
		pinyin_raw.for_each(|pinyin| {
			if let Some(pinyin) = pinyin {
				result.push(
					Some(Pin::new(pinyin))
				);
			}
			else {
				result.push(None);
			}
		});
		result
	}
	pub fn wrap(&self, max_width: u16, consider_pinyin: bool) -> Vec<&[Character]> {
		let mut result = Vec::new();
		let mut width = 0;
		let mut slow = 0;
		let mut fast = 0;
		for c in &self.chars {
			// break line when the line width exceeds the max_width
			// if `consider_pinyin`: A character's width will be max(char width, pinyin width + 1);
			// 'shang '
			// '上    '  =>    width = 6
			let mut char_width= c.char.width_cjk().unwrap_or(1);
			if consider_pinyin {
				if let Some(pinyin) = c.pinyin.as_ref() {
					char_width = cmp::max(char_width, pinyin.pinyin.len() + 1);
				}
			}
			if width + char_width > max_width.into() {
				result.push(&self.chars[slow..=fast]);
				width = 0;
				slow = fast + 1;
			}
			fast += 1;
			width += char_width;
		}
		// dbg!(&self.chars[slow..fast]);
		result.push(&self.chars[slow..fast]);
		result
	}
}

impl Debug for Passage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut string = String::new();
		self.chars.iter().for_each(|c| {
			if c.is_mark {
				write!(&mut string, "{} ", c.char).unwrap();
			}
			else {
				write!(&mut string, " {} {} ", c.char, c.pinyin.as_ref().unwrap().pinyin_with_tone).unwrap();
			}
			
		});
        write!(f, "{}",
			string
		)
    }
}

pub enum CharStatus {
	Passed,
	Typing,
	TypingHalf,
	ErrorConsonant,
	ErrorVowel,
	Future,
}

pub struct Character {
	pub char: char,
	pub pinyin: Option<Pin>,
	pub is_mark: bool,
	pub status: CharStatus,
}

impl Character {
	pub fn new(char: char, pinyin: Option<Pin>) -> Self {
		Self {
			is_mark: pinyin.is_none(),
			char,
			pinyin,
			status: CharStatus::Future,
		}
	}

	pub fn set_status(&mut self, status: CharStatus) {
		self.status = status;
	}

	pub fn char_style(&self, is_live: bool) -> impl Command {
		if is_live {
			match self.status {
				CharStatus::Passed			=> PrintStyledContent(self.char.with(Color::Green)),
				CharStatus::Typing			=> PrintStyledContent(self.char.with(Color::Blue)),
				CharStatus::TypingHalf		=> PrintStyledContent(self.char.with(Color::Cyan)),
				CharStatus::ErrorConsonant	|
				CharStatus::ErrorVowel		=> PrintStyledContent(self.char.with(Color::Red)),
				CharStatus::Future			=> PrintStyledContent(self.char.with(Color::White)),
			}
		}
		else {
			match self.status {
				CharStatus::Passed => PrintStyledContent(self.char.with(Color::DarkGreen)),
				CharStatus::Future => PrintStyledContent(self.char.with(Color::Grey)),
				/*Invalid*/ _ => PrintStyledContent(self.char.with(Color::Grey)),
			}
		}
	}

	pub fn pinyin_style(&self, is_live: bool) -> [impl Command;2] {
		let pinyin = if self.is_mark { String::new() } else { self.pinyin.as_ref().unwrap().pinyin_with_tone.clone() };
		let consonant = if self.is_mark { String::new() } else { self.pinyin.as_ref().unwrap().pinyin_splitted[0].clone() };
		let vowel = pinyin[consonant.len()..pinyin.len()].to_string();
		if is_live {
			match self.status {
				/*🟩🟩*/ CharStatus::Passed			 => [PrintStyledContent(consonant.with(Color::Green)),	PrintStyledContent(vowel.with(Color::Green).bold())],
				/*🟦🟦*/ CharStatus::Typing			 => [PrintStyledContent(consonant.with(Color::Blue )),	PrintStyledContent(vowel.with(Color::Blue ).bold())],
				/*🟩🟦*/ CharStatus::TypingHalf		 => [PrintStyledContent(consonant.with(Color::Green)),	PrintStyledContent(vowel.with(Color::Blue ).bold())],
				/*🟥🟦*/ CharStatus::ErrorConsonant	 => [PrintStyledContent(consonant.with(Color::Red  )),	PrintStyledContent(vowel.with(Color::Green).bold())],
				/*🟩🟥*/ CharStatus::ErrorVowel		 => [PrintStyledContent(consonant.with(Color::Green)),	PrintStyledContent(vowel.with(Color::Red  ).bold())],
				/*⬜⬜*/ CharStatus::Future		   => [PrintStyledContent(consonant.with(Color::White)),  PrintStyledContent(vowel.with(Color::White).bold())],
			}
		}
		else {
			match self.status {
				/*🟩🟩*/ CharStatus::Passed	=> [PrintStyledContent(consonant.with(Color::DarkGreen)),	PrintStyledContent(vowel.with(Color::DarkGreen).bold())],
				/*⬜⬜*/ CharStatus::Future => [PrintStyledContent(consonant.with(Color::Grey )),  PrintStyledContent(vowel.with(Color::Grey ).bold())],
				/*Invalid*/ _ => [PrintStyledContent(consonant.with(Color::Grey)),  PrintStyledContent(vowel.with(Color::Grey))],
			}
		}
	}
}


#[derive(Debug)]
pub struct Pin {
	pub pinyin_with_tone: String,
	pub pinyin: String,
	pub pinyin_splitted: [String;2]
}

impl Pin {
	pub fn new(pinyin: Pinyin) -> Self{
		Self {
			pinyin_with_tone: pinyin.with_tone().to_string(),
			pinyin_splitted: Self::split(pinyin.plain().to_string()),
			pinyin: pinyin.plain().to_string(),
		}
	}
	fn split(pinyin: String) -> [String;2] {
		let mut result: [String;2] = [String::new(), String::new()];
		let mut vowel_start = false;
		for phoneme in pinyin.chars() {
			match phoneme {
				'a' | 'e' | 'i' | 'o' | 'u' | 'v' | 'ü' => {
					vowel_start = true;
					result[1].push(phoneme);
				},
				_ => {
					if vowel_start {
						result[1].push(phoneme);
					}
					else {
						result[0].push(phoneme);
					}
				},
			}
		}
		result
	}
}
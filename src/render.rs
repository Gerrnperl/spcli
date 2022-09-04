use std::{io::{Write, stdout, Stdout, self}, cmp, collections::HashMap};
use crossterm::{terminal, ExecutableCommand, cursor::MoveTo, style::{Stylize, Color, PrintStyledContent, SetBackgroundColor}, QueueableCommand};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{pin::Passage, input::{KeyMap, Counter}};

const PADDING_TOP: u16 = 2;
const PADDING_ASIDE: u16 = 10;
const MAX_WIDTH: u16 = 120;

const MISTAKE_LABEL: &str = "错误: ";
const SPEED_LABEL: &str = "速度: ";
const TIME_LABEL: &str = "耗时: ";
const LABEL_LEN:i32 = 19;

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ToneType {
	// ni hao a
	// 你 好  啊
	Always,
	//   hao
	// 你好啊
	Live,
	// 你好啊
	Off
}

struct Rect {
	width: u16,
	height: u16,
	left: u16,
	top: u16,
}

pub struct Render {
	stdout: Stdout,
	passage_rect: Rect,
}

impl Render {
	pub fn new() -> Self {
		Self {
			stdout: stdout(),
			passage_rect: Self::calc_passage_rect(),
		}
	}
	pub fn refresh(&mut self) -> Result<(), io::Error> {
		self.stdout.execute(terminal::Clear(terminal::ClearType::All))?;
		Ok(())
	}

	pub fn render_passage(&mut self, passage: &Passage, passed: usize, tone_on: ToneType) -> Result<(), io::Error> {
		self.render_passage_border()?;
		let mut x = self.passage_rect.left;
		let mut y = self.passage_rect.top;
		let mut rendered_length = 0;
		let lines = passage.wrap(self.passage_rect.width, tone_on == ToneType::Always);

		for line in lines.iter() {
			let rendering_typing_line = rendered_length <= passed + 1&& rendered_length + line.len() > passed;
			if rendering_typing_line {
				self.stdout.
					queue(MoveTo(x.saturating_sub(2), if tone_on == ToneType::Off {y} else {y + 1}))?.
					queue(PrintStyledContent("┃".with(Color::Cyan)))?;
			}

			for (col, c) in line.iter().enumerate() {
				self.goto(x, y)?;
				
				match tone_on {
					ToneType::Always => {
						let mut char_width = c.char.width_cjk().unwrap_or(1);
						let pinyin = c.pinyin_style(rendering_typing_line);
						if !c.is_mark {
							char_width = cmp::max(char_width, c.pinyin.as_ref().unwrap().pinyin.len() + 1);
						}
						self.stdout.
						/*Pinyin*/	queue(&pinyin[0])?.
						/*Pinyin*/	queue(&pinyin[1])?.
						/* Text */	queue(MoveTo(x, y + 1))?.
									queue(c.char_style(rendering_typing_line))?;
						x += char_width as u16;
					},
					ToneType::Live => {
						let char_width = c.char.width_cjk().unwrap_or(1);
						if rendering_typing_line {
							if rendered_length + col == passed + 1 && !c.is_mark {
								let pinyin = c.pinyin_style(rendering_typing_line);
								self.stdout.
								/*Pinyin*/	queue(&pinyin[0])?.
								/*Pinyin*/	queue(&pinyin[1])?;
							}
							self.stdout.queue(MoveTo(x, y + 1))?;
						}
						self.stdout.queue(c.char_style(rendering_typing_line))?;
						x += char_width as u16;
					},
					ToneType::Off => {
						let char_width = c.char.width_cjk().unwrap_or(1);
						self.stdout.queue(c.char_style(rendering_typing_line))?;
						x += char_width as u16;
					},
				}
			};
			self.stdout.flush()?;
			rendered_length += line.len();
			x = self.passage_rect.left;
			y += if rendering_typing_line {
				 	 if tone_on == ToneType::Off { 1 } else { 2 }
				 }
				 else if tone_on == ToneType::Always { 2 } else { 1 };
		};
		Ok(())
	}

	pub fn render_passage_border(&mut self) -> Result<(), io::Error> {
		let Rect{width: mut w, height: mut h, top: mut y, left: mut x} = self.passage_rect;
		w += 4;
		h += 2;
		y = y.saturating_sub(1);
		x = x.saturating_sub(2);
		self.stdout.
			queue(MoveTo(x, y))?.
			queue(PrintStyledContent(format!("{}{}{}", "┌", "─".repeat(w.into()), "┐").with(Color::DarkBlue)))?.
			queue(MoveTo(x, y + h))?.
			queue(PrintStyledContent(format!("{}{}{}", "└", "─".repeat(w.into()), "┘").with(Color::DarkBlue)))?.
			queue(MoveTo(0, y + 1))?.
			queue(PrintStyledContent(
				format!(
					"{}│{}│\n\r",
					" ".repeat(x.into()),
					" ".repeat(w.into())
				).repeat((h-1).into()).with(Color::DarkBlue))
			)?;
		Ok(())
	}
	fn goto(&mut self, x: u16, y: u16) -> Result<(), io::Error> {
		self.stdout.execute(MoveTo(x, y))?;
		Ok(())
	}

	fn calc_passage_rect() -> Rect {
		let max_height = if terminal::size().unwrap().1 > 27 { 
			terminal::size().unwrap().1.saturating_div(2) - PADDING_TOP
		}
		else {
			terminal::size().unwrap().1 - PADDING_TOP * 2
		};
		let mut max_width = terminal::size().unwrap().0;
		let scaled_aside;
		if max_width > MAX_WIDTH + 8 {				// window is too wide
			scaled_aside = (max_width - MAX_WIDTH) / 2;
			max_width = MAX_WIDTH;
		}
		else if max_width > PADDING_ASIDE * 4 { // window is not too small
			scaled_aside = max_width / MAX_WIDTH * PADDING_ASIDE + 4;
			max_width -= scaled_aside * 2
		}
		else {									// window is too small
			scaled_aside = 4;
			max_width -= scaled_aside * 2
		}
		Rect {
			width: max_width,
			height: max_height,
			top: PADDING_TOP,
			left: scaled_aside,
		}
	}

	pub fn update_passage_rect(&mut self) {
		self.passage_rect = Self::calc_passage_rect();
	}

	pub fn render_keyboard(&mut self, keyboard: &Keyboard, highlight_rule: HashMap<String, Color>) -> Result<(), io::Error> {
		let max_width = keyboard.layout.iter().fold(0, |max, row| {cmp::max(max, row.len())}) as u16;
		let max_height = keyboard.layout.len() as u16;
		let terminal_width = terminal::size().unwrap().0;
		let terminal_height = terminal::size().unwrap().1;
		// dbg!(max_width, terminal_width);
		if terminal_width < max_width * 9 || terminal_height / 2 - PADDING_TOP < max_height * 4 {
			return Ok(());
		}
		let x = (terminal_width.saturating_sub(max_width * 9)) / 2;
		let y = self.passage_rect.top + self.passage_rect.height + PADDING_TOP;
		let offset = 2;
		self.goto(x, y)?;
		keyboard.key_map.values().for_each(|key| {
			key.render(&mut self.stdout, x + key.position.0 * 9 + key.position.1 * offset, y + key.position.1 * 4, highlight_rule.get(&key.name)).unwrap();
		});
		self.stdout.flush()?;
		Ok(())
	}

	pub fn render_counter(&mut self, counter: &Counter) -> Result<(), io::Error> {
		let interval = counter.get_interval().as_secs();
		let terminal_width = terminal::size().unwrap().0;
		let key_map_name = counter.get_key_map_name();
		let typed_str = format!("  {}字", counter.get_typed_words());
		let total_str = format!("{}字", counter.get_total_words());
		let percent_str = format!("{:.1}%", counter.get_typed_words() as f32 / counter.get_total_words() as f32 * 100f32);
		let mistakes_str = format!("{}", counter.get_mistakes());
		let speed_str =
			if interval == 0 {
				String::from("NaN字/min")
			}
			else {
				format!("{}字/min", counter.get_typed_words() * 60u32 / interval as u32 )
			};
		let suggestions_str = {
			if interval < 5 {
				String::from("Press <C-q> to quit, <C-r> to try again")
			}
			else {
				String::new()
			}
		};
		let time_str = format!("{:02}:{:02}  ", interval / 60, interval % 60);
		let gap_length =
			terminal_width as i32
			- key_map_name.len() as i32
			- typed_str.len() as i32
			- total_str.len() as i32
			- percent_str.len() as i32
			- suggestions_str.len() as i32
			- mistakes_str.len() as i32
			- speed_str.len() as i32
			- time_str.len() as i32
			- LABEL_LEN;
		if gap_length < 0 {
			return Ok(());
		}
		self.stdout.
			queue(MoveTo(0,0))?.
			queue(SetBackgroundColor(Color::Rgb { r: 42, g: 47, b: 49 }))?.
			queue(PrintStyledContent(typed_str.with(Color::White)))?.
			queue(PrintStyledContent("/".with(Color::Grey)))?.
			queue(PrintStyledContent(total_str.with(Color::White)))?.
			queue(PrintStyledContent("│".with(Color::Blue)))?.
			queue(PrintStyledContent(percent_str.with(Color::White)))?.
			queue(PrintStyledContent("│".with(Color::Blue)))?.
			queue(PrintStyledContent(key_map_name.with(Color::White)))?.
			queue(PrintStyledContent("│".with(Color::Blue)))?.
			queue(PrintStyledContent(suggestions_str.with(Color::White)))?.
			queue(PrintStyledContent(" ".repeat(gap_length.try_into().unwrap()).with(Color::White)))?.
			queue(PrintStyledContent(MISTAKE_LABEL.with(Color::DarkRed)))?.
			queue(PrintStyledContent(mistakes_str.with(Color::Red)))?.
			queue(PrintStyledContent("│".with(Color::Blue)))?.
			queue(PrintStyledContent(SPEED_LABEL.with(Color::White)))?.
			queue(PrintStyledContent(speed_str.with(Color::White)))?.
			queue(PrintStyledContent("│".with(Color::Blue)))?.
			queue(PrintStyledContent(TIME_LABEL.with(Color::White)))?.
			queue(PrintStyledContent(time_str.with(Color::White)))?.
			queue(SetBackgroundColor(Color::Reset))?;
		self.stdout.flush()?;
		Ok(())
	}

	pub fn render_summary(&mut self, counter: &Counter) -> Result<(), io::Error> {
		self.refresh()?;
		self.render_passage_border()?;
		let x = self.passage_rect.left;
		let y = self.passage_rect.top;
		let w = self.passage_rect.width as usize;
		let interval = counter.get_interval().as_secs();
		let mistakes_str = format!("错误: {}  ", counter.get_mistakes());
		let speed_str =
			if interval == 0 {
				String::from("速度: NaN字/min")
			}
			else {
				format!("速度: {}字/min", counter.get_typed_words() * 60u32 / interval as u32 )
			};
		let time_str = format!("耗时: {:02}:{:02}  ", interval / 60, interval % 60);
		self.stdout.
			queue(MoveTo(x+2, y+1))?.
			queue(PrintStyledContent(format!("{:<width$}", "🎉🎉 完成 🎉🎉", width=w-5).with(Color::Yellow)))?.
			queue(MoveTo(x+2, y+3))?.
			queue(PrintStyledContent(format!("{:<width$}", speed_str, width=w-2).with(Color::Cyan)))?.
			queue(MoveTo(x+2, y+4))?.
			queue(PrintStyledContent(format!("{:<width$}", time_str, width=w-2).with(Color::Cyan)))?.
			queue(MoveTo(x+2, y+5))?.
			queue(PrintStyledContent(format!("{:<width$}", mistakes_str, width=w-2).with(Color::Red)))?.
			queue(MoveTo(x+2, y+7))?.
			queue(PrintStyledContent(format!("{:<width$}", "Press <C-q> to quit, <C-r> to try again", width=w).with(Color::DarkYellow)))?.
			flush()?;
		Ok(())
	}
	
}

pub struct Keyboard<'a> {
	key_map: HashMap<char, Key<'a>>,
	layout: [Vec<char>;3],
}

impl<'a> Keyboard<'a> {
	pub fn new(key_map: &'a KeyMap, layout:[Vec<char>;3]) -> Self {

		Self {
			key_map: Self::reverse_mapping(key_map, layout.clone()),
			layout,
		}
	}

	pub fn default(key_map: &'a KeyMap) -> Self {
		Self::new(key_map, [
			vec!['q','w','e','r','t','y','u','i','o','p'],
			 vec!['a','s','d','f','g','h','j','k','l',';'],
			  vec!['z','x','c','v','b','n','m']
		])
	}

	fn reverse_mapping(key_map: &KeyMap, layout: [Vec<char>;3]) -> HashMap<char, Key> {
		let map = &key_map.map;
		let mut reversed_key_map:HashMap<char, Vec<String>> = HashMap::new();
		let mut result: HashMap<char, Key> = HashMap::new();
		map.iter().for_each(|(phoneme, keys)| {
			keys.iter().for_each(|key| {
				if let Some(v) = reversed_key_map.get_mut(key) {
					v.push(phoneme.to_string());
				}
				else {
					reversed_key_map.insert(*key, vec![phoneme.to_string()]);
				}
			})
		});
		for (y, row) in layout.iter().enumerate() {
			for (x, key) in row.iter().enumerate() {
				let right = row.len() - 1;
				let bottom = layout.len() - 1;
				let key_border_top = 
					if y == 0 && x == right  {key_border::top::NE} 
					else if y == 0 && x == 0 {key_border::top::NW} 
					else if y == 0 			 {key_border::top::N } 
					else if x == right 		 {key_border::top::E }
					else if x == 0			 {key_border::top::W } 
					else 					 {key_border::top::CT};
				let key_border_bottom =
					if (y == bottom && x == right)
					|| (y != bottom && x > layout.get(y + 1).unwrap().len()) {key_border::bottom::SE}
					else if y == bottom && x == 0 {key_border::bottom::SW}
					else if y == bottom 		  {key_border::bottom::S }
					else if x == right			  {key_border::bottom::E }
					else if x == 0				  {key_border::bottom::W }
					else 						  {key_border::bottom::CT};
				dbg!(key);
				result.insert(
					*key,
					Key::new(
						key_border_top,
						key_border_bottom,
						key.to_string(),
						reversed_key_map.get(key).unwrap_or(&Vec::new()).clone(),
						(x as u16, y as u16)
					),
				);
			}
		}
		result
	}
}

struct Key<'a> {
	top: &'a str,
	bottom: &'a str,
	name: String,
	phonemes: Vec<String>,
	position: (u16, u16),
}

impl<'a> Key<'a> {
	pub fn new(top: &'a str, bottom: &'a str, name: String, phonemes: Vec<String>, position: (u16, u16)) -> Self {
		Self {top, bottom, name, phonemes, position}
	}

	pub fn render(&self, stdout: &mut Stdout, x: u16, y: u16, highlight: Option<&Color>) -> Result<(), io::Error> {
		stdout.
		queue(MoveTo(x, y))?.
		queue(PrintStyledContent(self.top.with(Color::DarkBlue)))?.
			queue(MoveTo(x+1, y+1))?.
			queue(PrintStyledContent(format!(" {:<7}", self.name).with(Color::Green)))?.
			queue(MoveTo(x, y+1))?.
			queue(PrintStyledContent("│".with(Color::DarkBlue)))?.
			queue(MoveTo(x+9, y+1))?.
			queue(PrintStyledContent("│".with(Color::DarkBlue)))?.
			queue(MoveTo(x, y+2))?.
			queue(PrintStyledContent("│".with(Color::DarkBlue)))?.
			queue(MoveTo(x+9, y+2))?.
			queue(PrintStyledContent("│".with(Color::DarkBlue)))?.
			queue(MoveTo(x, y+3))?.
			queue(PrintStyledContent("│".with(Color::DarkBlue)))?.
			queue(MoveTo(x+9, y+3))?.
			queue(PrintStyledContent("│".with(Color::DarkBlue)))?.
			queue(MoveTo(x, y+4))?.
			queue(PrintStyledContent(self.bottom.with(Color::DarkBlue)))?;


		if highlight.is_some() {
			stdout.queue(SetBackgroundColor(*highlight.unwrap()))?;
		}
		stdout.
			queue(MoveTo(x+1, y+1))?.
			queue(PrintStyledContent(format!(" {:<7}", self.name).with(Color::Green)))?.
			queue(MoveTo(x+1, y+2))?.
			queue(PrintStyledContent("        ".with(Color::DarkBlue)))?.
			queue(MoveTo(x+1, y+3))?.
			queue(PrintStyledContent("        ".with(Color::DarkBlue)))?;
		let mut right_count = 0;
		let mut left_count = 0;
		for phoneme in self.phonemes.iter() {
			if *phoneme == self.name && self.phonemes.len() > 1 && phoneme != "a" && phoneme != "e" && phoneme != "i" && phoneme != "o" && phoneme != "u"{
				continue;
			}
			match phoneme.as_str() {
				"zh" | "sh" | "ch" => {
					stdout.
						queue(MoveTo(x+1, y + 3 - left_count))?.
						queue(PrintStyledContent(format!(" {}", phoneme).with(Color::Red)))?;
					left_count += 1;
				},
				_ => {
					stdout.
						queue(MoveTo(x+8-phoneme.width() as u16, y + 3 - right_count))?.
						queue(PrintStyledContent(phoneme.clone().with(Color::Yellow)))?;
					right_count += 1;
				}
			}
		};
		if highlight.is_some() {
			stdout.queue(SetBackgroundColor(Color::Reset))?;
		}
		Ok(())
	}
}

mod key_border{
	pub mod top {
		pub const NW :&str = "┌────────┬";
		pub const N	 :&str = "┬────────┬";
		pub const NE :&str = "┬────────┐";
		pub const CT :&str = "┬──────┴─┬";
		pub const W  :&str = "┬──────┴─┬";
		pub const E	 :&str = "┬──────┴─┐";
	}
	pub mod bottom {
		pub const SW :&str = "└────────┴";
		pub const S  :&str = "┴────────┴";
		pub const SE :&str = "┴────────┘";
		pub const CT :&str = "┴─┬──────┴";
		pub const W  :&str = "└─┬──────┴";
		pub const E  :&str = "┴─┬──────┘";
	} 
}
use std::{io, time::{Duration, Instant}, fs, collections::HashMap};

use crossterm::{terminal, event, style::Color};

use crate::{die, pin::{Document, CharStatus}, render::{Render, ToneType, Keyboard}};
pub struct Input<'b> {
	document: &'b mut Document,
	active_passage: usize,
	render: Render,
	passed: usize,
	typing_consonant: bool,
	key_map: &'b KeyMap,
	keyboard: Keyboard<'b>,
	counter: Counter,
	stopped: bool,
	restart: bool,
	end: bool,
	tone_on: ToneType,
}

impl<'b> Input<'b> {
	pub fn new(document: &'b mut Document, key_map: &'b KeyMap, tone_on: ToneType) -> Self {
		let rnd = document.get_random();
		let passage = document.passages.get(rnd).unwrap();
		let total_words = passage.chars.iter().fold(0, |acc, char| { if !char.is_mark { acc + 1 } else { acc } });
		Self {
    		active_passage: rnd,
			document,
			render: Render::new(),
			passed: 0,
			typing_consonant: true,
			keyboard: Keyboard::default(key_map),
			key_map,
			counter: Counter::new(total_words, key_map.name.clone()),
			stopped: false,
			restart: false,
			end: false,
			tone_on,
		}
	}

	pub fn run(&mut self) -> bool {
		// Enable Raw Mode
		if !terminal::is_raw_mode_enabled().expect("Can not read if raw mode is enabled") {
			terminal::enable_raw_mode().expect("Failed to enable raw mode");
		}

		// init
		self.render.refresh().unwrap();
		let passage = self.document.passages.get(self.active_passage).unwrap();
		self.render.render_passage(passage, self.passed, self.tone_on).unwrap();
		self.render.render_keyboard(&self.keyboard, HashMap::new()).unwrap();
		self.render.render_counter(&self.counter).unwrap();

		while !self.stopped {
			if event::poll(Duration::from_millis(1000)).unwrap() {
				// It's guaranteed that `read_input` won't block
				if let Err(err) = self.read_input() {
					eprintln!("{}", err);
					die();
					break;
				}
			}
			else if !self.stopped {
				self.render.render_counter(&self.counter).unwrap();
			}
		}
		self.restart

	}

	pub fn read_input(&mut self) -> Result<(), io::Error> {
		let input = event::read()?;
		match input {
			event::Event::Key(key) => {
				self.process_key_event(key)?;
			},
			event::Event::Resize(_, _) => {
				self.render.update_passage_rect();
				self.render.refresh()?;
				let passage = self.document.passages.get(self.active_passage).unwrap();
				self.render.render_passage(passage, self.passed, self.tone_on)?;
				self.render.render_keyboard(&self.keyboard, HashMap::new())?;
			},
			_ => ()
		}

		self.render.render_counter(&self.counter)?;
		Ok(())
	}

	fn process_key_event(&mut self, event: event::KeyEvent) -> Result<(), io::Error> {
		match event.modifiers {
			event::KeyModifiers::CONTROL => self.process_ctrl_key_event(event),
			event::KeyModifiers::NONE => {
				if self.end {
					return Ok(()); 
				}
				if let event::KeyCode::Char(character) = event.code {
					self.check_input(character)?;
				}
				Ok(())
			}
			_ => Ok(())
		}
	}

	fn process_ctrl_key_event(&mut self, event: event::KeyEvent) -> Result<(), io::Error> {
		match event.code {
			event::KeyCode::Char('q') => {
				die();
			},
			event::KeyCode::Char('r') => {
				self.stopped = true;
				self.restart = true;
			},
			_ => ()
		}
		Ok(())
	}

	fn check_input(&mut self, character: char) -> Result<(), io::Error> {
		let passage = self.document.passages.get_mut(self.active_passage).unwrap();
		// Turn the color of the auto-passed mark to green;
		while passage.chars.get(self.passed + 1).unwrap().is_mark {
			passage.chars.get_mut(self.passed + 1).unwrap().set_status(CharStatus::Passed);
			self.passed += 1;
		}

		let typing = passage.chars.get_mut(self.passed + 1).unwrap();
		let pinyin = &typing.pinyin.as_ref().unwrap().pinyin_splitted;
		let map = &self.key_map.map;
		// Check if the key is correct
		let pass = 
			if self.typing_consonant {
				let expected = &pinyin[0];
				if pinyin[0].is_empty() && pinyin[1] == "er" && self.key_map.split_er && character != self.key_map.leader{
					// 处理特殊的"er"
					map.get("e").unwrap().contains(&character)
				}
				else if expected.is_empty() {
					// 零声母
					match self.key_map.leader {
						'*' => map.get(&pinyin[1][0..1]).unwrap().contains(&character),
						c => character == c,
					}
				}
				else {
					map.get(expected).unwrap().contains(&character)
				}
			}
			else {
				let expected = &pinyin[1];
				if pinyin[0].is_empty() && pinyin[1] == "er" && self.key_map.split_er && character != self.key_map.leader{
					map.get("r").unwrap().contains(&character)
				}
				else {
					map.get(expected).unwrap().contains(&character)
				}
			};

		if self.typing_consonant {
			if pass {
				typing.set_status(CharStatus::TypingHalf);
				self.typing_consonant = false;
			}
			else {
				typing.set_status(CharStatus::ErrorConsonant);
				self.counter.add_mistake();
			}
		}
		else if pass {
			typing.set_status(CharStatus::Passed);
			self.passed += 1;
			self.counter.add_typed_words();
			if self.passed >= passage.chars.len() - 2 {
				// END
				self.end = true;
				self.counter.lock();
				self.render.render_summary(&self.counter)?;
				return Ok(());
			}
			else {
				loop {
					if let Some(character) = passage.chars.get(self.passed + 1) {
						if character.is_mark {
							passage.chars.get_mut(self.passed + 1).unwrap().set_status(CharStatus::Passed);
							self.passed += 1;
						}
						else {
							break;
						}
					}
					else {
						// END
						self.counter.lock();
						self.end = true;
						self.render.render_summary(&self.counter)?;
						return Ok(());
					}
				}
			}
			self.typing_consonant = true;
			passage.chars.get_mut(self.passed + 1).unwrap().set_status(CharStatus::Typing);
		}
		else {
			typing.set_status(CharStatus::ErrorVowel);
			self.counter.add_mistake();
		}
		


		// Rerender thr keyboard
		let next = {
			let typing = passage.chars.get(self.passed+1).unwrap();
			let pinyin = &typing.pinyin.as_ref().unwrap().pinyin_splitted;
			let mut phoneme = pinyin[if self.typing_consonant {0} else {1}].clone();
			if phoneme.is_empty() {
				phoneme = match self.key_map.leader {
					'*' => {
						let phoneme = &pinyin[1][0..1];
						phoneme.to_string()
					},
					c => String::from(c),
				};
			}
			map.get(&phoneme)
		};

		let mut highlight_rule = HashMap::from([
			(character.to_string(), if pass {Color::DarkGreen} else {Color::DarkRed}),
		]);

		if let Some(keys) = next {
			for key in keys {
				highlight_rule.insert(key.to_string(), Color::DarkBlue);
			}
		}

		self.render.refresh()?;
		self.render.render_passage(passage, self.passed, self.tone_on)?;
		self.render.render_keyboard(&self.keyboard, highlight_rule)?;

		Ok(())
	}
}

pub struct KeyMap {
	leader: char,
	name: String,
	pub map: HashMap<String, Vec<char>>,
	split_er: bool,
}

impl KeyMap {
	pub fn open(filepath: &str) -> Result<KeyMap, io::Error> {
		let file = fs::read_to_string(filepath)?;
		let mut map = HashMap::new();
		let mut leader = 'o';
		let mut split_er = true;
		let mut name:String = String::new();
		for line in file.split('\n') {
			if line.trim().starts_with('#') || line.trim().is_empty() {
				continue;
			}
			let split: Vec<&str> = line.split(&[':'][..]).collect();
			let phoneme = split[0].trim().to_owned();
			let key:Vec<char> = split[1].trim().chars().collect();

			if phoneme == "leader" {
				leader = key[0];
			}
			else if phoneme == "leader_er" {
				map.insert("er".to_owned(), vec![key[0]]);
			}
			else if phoneme == "split_er" {
				split_er = key[0] == '1';
			}
			else if phoneme == "name" {
				name = split[1].trim().to_string();
			}
			else {
				map.insert(phoneme, key);
			}
		}
		Ok(Self {
			leader,
			map,
			split_er,
			name
		})
	}
}

pub struct Counter {
	total_words: u32,
	typed_words: u32,
	mistakes: u32,
	start_time: Instant,
	time_locked: bool, 
	end_time: Instant,
	key_map_name: String,
}

impl Counter {
	pub fn new(total_words: u32, key_map_name: String) -> Self {
		Self {
			total_words,
			typed_words: 0,
			mistakes: 0,
			start_time: Instant::now(),
			end_time: Instant::now(),
			time_locked: false,
			key_map_name,
		}
	}

	pub fn add_typed_words(&mut self) {
		self.typed_words += 1;
	}

	pub fn add_mistake(&mut self) {
		self.mistakes += 1;
	}

	pub fn get_total_words(&self) -> u32 {
		self.total_words
	}

	pub fn get_typed_words(&self) -> u32 {
		self.typed_words
	}

	pub fn get_mistakes(&self) -> u32 {
		self.mistakes
	}

	pub fn get_key_map_name(&self) -> String {
		self.key_map_name.clone()
	}

	pub fn get_interval(&self) -> Duration {
		if self.time_locked {
			self.end_time.duration_since(self.start_time)
		}
		else {
			Instant::now().duration_since(self.start_time)
		}
	}

	pub fn lock(&mut self) {
		self.time_locked = true;
		self.end_time = Instant::now();
	}

}
//TODO: remove once fixed https://github.com/rust-lang/rust-clippy/issues/6818
#![allow(clippy::use_self)]

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ron::{
	self,
	ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};
use std::{
	fs::{self, File},
	io::{Read, Write},
	path::PathBuf,
	rc::Rc,
};

use crate::{args::get_app_config_path, strings::symbol};

pub type SharedKeyConfig = Rc<KeyConfig>;

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyConfig {
	pub tab_status: KeyEvent,
	pub tab_log: KeyEvent,
	pub tab_files: KeyEvent,
	pub tab_stashing: KeyEvent,
	pub tab_stashes: KeyEvent,
	pub tab_toggle: KeyEvent,
	pub tab_toggle_reverse: KeyEvent,
	pub toggle_workarea: KeyEvent,
	pub focus_right: KeyEvent,
	pub focus_left: KeyEvent,
	pub focus_above: KeyEvent,
	pub focus_below: KeyEvent,
	pub exit: KeyEvent,
	pub quit: KeyEvent,
	pub exit_popup: KeyEvent,
	pub open_commit: KeyEvent,
	pub open_commit_editor: KeyEvent,
	pub open_help: KeyEvent,
	pub open_options: KeyEvent,
	pub move_left: KeyEvent,
	pub move_right: KeyEvent,
	pub tree_collapse_recursive: KeyEvent,
	pub tree_expand_recursive: KeyEvent,
	pub home: KeyEvent,
	pub end: KeyEvent,
	pub move_up: KeyEvent,
	pub move_down: KeyEvent,
	pub page_down: KeyEvent,
	pub page_up: KeyEvent,
	pub shift_up: KeyEvent,
	pub shift_down: KeyEvent,
	pub enter: KeyEvent,
	pub blame: KeyEvent,
	pub file_history: KeyEvent,
	pub edit_file: KeyEvent,
	pub status_stage_all: KeyEvent,
	pub status_reset_item: KeyEvent,
	pub status_ignore_file: KeyEvent,
	pub diff_stage_lines: KeyEvent,
	pub diff_reset_lines: KeyEvent,
	pub stashing_save: KeyEvent,
	pub stashing_toggle_untracked: KeyEvent,
	pub stashing_toggle_index: KeyEvent,
	pub stash_apply: KeyEvent,
	pub stash_open: KeyEvent,
	pub stash_drop: KeyEvent,
	pub cmd_bar_toggle: KeyEvent,
	pub log_tag_commit: KeyEvent,
	pub log_mark_commit: KeyEvent,
	pub commit_amend: KeyEvent,
	pub copy: KeyEvent,
	pub create_branch: KeyEvent,
	pub rename_branch: KeyEvent,
	pub select_branch: KeyEvent,
	pub delete_branch: KeyEvent,
	pub merge_branch: KeyEvent,
	pub rebase_branch: KeyEvent,
	pub compare_commits: KeyEvent,
	pub tags: KeyEvent,
	pub delete_tag: KeyEvent,
	pub select_tag: KeyEvent,
	pub push: KeyEvent,
	pub open_file_tree: KeyEvent,
	pub force_push: KeyEvent,
	pub pull: KeyEvent,
	pub abort_merge: KeyEvent,
	pub undo_commit: KeyEvent,
}

#[rustfmt::skip]
impl Default for KeyConfig {
	fn default() -> Self {
		Self {
			tab_status: KeyEvent { code: KeyCode::Char('1'), modifiers: KeyModifiers::empty()},
			tab_log: KeyEvent { code: KeyCode::Char('2'), modifiers: KeyModifiers::empty()},
			tab_files: KeyEvent { code: KeyCode::Char('3'), modifiers: KeyModifiers::empty()},
			tab_stashing: KeyEvent { code: KeyCode::Char('4'), modifiers: KeyModifiers::empty()},
			tab_stashes: KeyEvent { code: KeyCode::Char('5'), modifiers: KeyModifiers::empty()},
			tab_toggle: KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::empty()},
			tab_toggle_reverse: KeyEvent { code: KeyCode::BackTab, modifiers: KeyModifiers::SHIFT},
			toggle_workarea: KeyEvent { code: KeyCode::Char('w'), modifiers: KeyModifiers::empty()},
			focus_right: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty()},
			focus_left: KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::empty()},
			focus_above: KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::empty()},
			focus_below: KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::empty()},
			exit: KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL},
			quit: KeyEvent { code: KeyCode::Char('q'), modifiers: KeyModifiers::empty()},
			exit_popup: KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::empty()},
			open_commit: KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::empty()},
			open_commit_editor: KeyEvent { code: KeyCode::Char('e'), modifiers:KeyModifiers::CONTROL},
			open_help: KeyEvent { code: KeyCode::Char('h'), modifiers: KeyModifiers::empty()},
			open_options: KeyEvent { code: KeyCode::Char('o'), modifiers: KeyModifiers::empty()},
			move_left: KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::empty()},
			move_right: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty()},
			tree_collapse_recursive: KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::SHIFT},
			tree_expand_recursive: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::SHIFT},
			home: KeyEvent { code: KeyCode::Home, modifiers: KeyModifiers::empty()},
			end: KeyEvent { code: KeyCode::End, modifiers: KeyModifiers::empty()},
			move_up: KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::empty()},
			move_down: KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::empty()},
			page_down: KeyEvent { code: KeyCode::PageDown, modifiers: KeyModifiers::empty()},
			page_up: KeyEvent { code: KeyCode::PageUp, modifiers: KeyModifiers::empty()},
			shift_up: KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::SHIFT},
			shift_down: KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::SHIFT},
			enter: KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::empty()},
			blame: KeyEvent { code: KeyCode::Char('B'), modifiers: KeyModifiers::SHIFT},
			file_history: KeyEvent { code: KeyCode::Char('H'), modifiers: KeyModifiers::SHIFT},
			edit_file: KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::empty()},
			status_stage_all: KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::empty()},
			status_reset_item: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
			diff_reset_lines: KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::empty()},
			status_ignore_file: KeyEvent { code: KeyCode::Char('i'), modifiers: KeyModifiers::empty()},
			diff_stage_lines: KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::empty()},
			stashing_save: KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::empty()},
			stashing_toggle_untracked: KeyEvent { code: KeyCode::Char('u'), modifiers: KeyModifiers::empty()},
			stashing_toggle_index: KeyEvent { code: KeyCode::Char('i'), modifiers: KeyModifiers::empty()},
			stash_apply: KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::empty()},
			stash_open: KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::empty()},
			stash_drop: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
			cmd_bar_toggle: KeyEvent { code: KeyCode::Char('.'), modifiers: KeyModifiers::empty()},
			log_tag_commit: KeyEvent { code: KeyCode::Char('t'), modifiers: KeyModifiers::empty()},
			log_mark_commit: KeyEvent { code: KeyCode::Char(' '), modifiers: KeyModifiers::empty()},
			commit_amend: KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::CONTROL},
			copy: KeyEvent { code: KeyCode::Char('y'), modifiers: KeyModifiers::empty()},
			create_branch: KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::empty()},
			rename_branch: KeyEvent { code: KeyCode::Char('r'), modifiers: KeyModifiers::empty()},
			select_branch: KeyEvent { code: KeyCode::Char('b'), modifiers: KeyModifiers::empty()},
			delete_branch: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
			merge_branch: KeyEvent { code: KeyCode::Char('m'), modifiers: KeyModifiers::empty()},
			rebase_branch: KeyEvent { code: KeyCode::Char('R'), modifiers: KeyModifiers::SHIFT},
			compare_commits: KeyEvent { code: KeyCode::Char('C'), modifiers: KeyModifiers::SHIFT},
			tags: KeyEvent { code: KeyCode::Char('T'), modifiers: KeyModifiers::SHIFT},
			delete_tag: KeyEvent { code: KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT},
			select_tag: KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::empty()},
			push: KeyEvent { code: KeyCode::Char('p'), modifiers: KeyModifiers::empty()},
			force_push: KeyEvent { code: KeyCode::Char('P'), modifiers: KeyModifiers::SHIFT},
			undo_commit: KeyEvent { code: KeyCode::Char('U'), modifiers: KeyModifiers::SHIFT},
			pull: KeyEvent { code: KeyCode::Char('f'), modifiers: KeyModifiers::empty()},
			abort_merge: KeyEvent { code: KeyCode::Char('M'), modifiers: KeyModifiers::SHIFT},
			open_file_tree: KeyEvent { code: KeyCode::Char('F'), modifiers: KeyModifiers::SHIFT},
		}
	}
}

impl KeyConfig {
	fn save(&self, file: PathBuf) -> Result<()> {
		let mut file = File::create(file)?;
		let data = to_string_pretty(self, PrettyConfig::default())?;
		file.write_all(data.as_bytes())?;
		Ok(())
	}

	pub fn get_config_file() -> Result<PathBuf> {
		let app_home = get_app_config_path()?;
		Ok(app_home.join("key_config.ron"))
	}

	fn read_file(config_file: PathBuf) -> Result<Self> {
		let mut f = File::open(config_file)?;
		let mut buffer = Vec::new();
		f.read_to_end(&mut buffer)?;
		Ok(ron::de::from_bytes(&buffer)?)
	}

	pub fn init(file: PathBuf) -> Result<Self> {
		if file.exists() {
			match Self::read_file(file.clone()) {
				Err(e) => {
					let config_path = file.clone();
					let config_path_old =
						format!("{}.old", file.to_string_lossy());
					fs::rename(
						config_path.clone(),
						config_path_old.clone(),
					)?;

					Self::default().save(file)?;

					Err(anyhow::anyhow!("{}\n Old file was renamed to {:?}.\n Defaults loaded and saved as {:?}",
						e,config_path_old,config_path.to_string_lossy()))
				}
				Ok(res) => Ok(res),
			}
		} else {
			Self::default().save(file)?;
			Ok(Self::default())
		}
	}

	//TODO: make this configurable (https://github.com/extrawurst/gitui/issues/465)
	#[allow(clippy::unused_self)]
	const fn get_key_symbol(&self, k: KeyCode) -> &str {
		match k {
			KeyCode::Enter => "\u{23ce}",     //⏎
			KeyCode::Left => "\u{2190}",      //←
			KeyCode::Right => "\u{2192}",     //→
			KeyCode::Up => "\u{2191}",        //↑
			KeyCode::Down => "\u{2193}",      //↓
			KeyCode::Backspace => "\u{232b}", //⌫
			KeyCode::Home => "\u{2912}",      //⤒
			KeyCode::End => "\u{2913}",       //⤓
			KeyCode::PageUp => "\u{21de}",    //⇞
			KeyCode::PageDown => "\u{21df}",  //⇟
			KeyCode::Tab => "\u{21e5}",       //⇥
			KeyCode::BackTab => "\u{21e4}",   //⇤
			KeyCode::Delete => "\u{2326}",    //⌦
			KeyCode::Insert => "\u{2380}",    //⎀
			KeyCode::Esc => "\u{238b}",       //⎋
			_ => "?",
		}
	}

	pub fn get_hint(&self, ev: KeyEvent) -> String {
		match ev.code {
			KeyCode::Down
			| KeyCode::Up
			| KeyCode::Right
			| KeyCode::Left
			| KeyCode::Enter
			| KeyCode::Backspace
			| KeyCode::Home
			| KeyCode::End
			| KeyCode::PageUp
			| KeyCode::PageDown
			| KeyCode::Tab
			| KeyCode::BackTab
			| KeyCode::Delete
			| KeyCode::Insert
			| KeyCode::Esc => {
				format!(
					"{}{}",
					Self::get_modifier_hint(ev.modifiers),
					self.get_key_symbol(ev.code)
				)
			}
			KeyCode::Char(' ') => String::from(symbol::SPACE),
			KeyCode::Char(c) => {
				format!(
					"{}{}",
					Self::get_modifier_hint(ev.modifiers),
					c
				)
			}
			KeyCode::F(u) => {
				format!(
					"{}F{}",
					Self::get_modifier_hint(ev.modifiers),
					u
				)
			}
			KeyCode::Null => Self::get_modifier_hint(ev.modifiers),
		}
	}

	//TODO: make customizable (see https://github.com/extrawurst/gitui/issues/465)
	fn get_modifier_hint(modifier: KeyModifiers) -> String {
		match modifier {
			KeyModifiers::CONTROL => "^".to_string(),
			KeyModifiers::SHIFT => {
				"\u{21e7}".to_string() //⇧
			}
			KeyModifiers::ALT => {
				"\u{2325}".to_string() //⌥
			}
			_ => String::new(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::KeyConfig;
	use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

	#[test]
	fn test_get_hint() {
		let config = KeyConfig::default();
		let h = config.get_hint(KeyEvent {
			code: KeyCode::Char('c'),
			modifiers: KeyModifiers::CONTROL,
		});
		assert_eq!(h, "^c");
	}

	#[test]
	fn test_load_vim_style_example() {
		assert_eq!(
			KeyConfig::read_file("vim_style_key_config.ron".into())
				.is_ok(),
			true
		);
	}
}

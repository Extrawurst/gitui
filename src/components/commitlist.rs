use super::utils::logitems::{ItemBatch, LogEntry};
use crate::{
	components::{
		utils::string_width_align, CommandBlocking, CommandInfo,
		Component, DrawableComponent, EventState, ScrollType,
	},
	keys::SharedKeyConfig,
	strings::{self, symbol},
	ui::style::{SharedTheme, Theme},
	ui::{self, calc_scroll_top},
};
use anyhow::Result;
use asyncgit::sync::{CommitId, Tags};
use chrono::{DateTime, Local};
use crossterm::event::Event;
use std::{
	borrow::Cow, cell::Cell, cmp, convert::TryFrom, time::Instant,
};
use tui::{
	backend::Backend,
	layout::{Alignment, Rect},
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph},
	Frame,
};

const ELEMENTS_PER_LINE: usize = 9;

///
pub struct CommitList {
	title: Box<str>,
	selection: usize,
	branch: Option<String>,
	count_total: usize,
	items: ItemBatch,
	marked: Vec<CommitId>,
	scroll_state: (Instant, f32),
	tags: Option<Tags>,
	current_size: Cell<(u16, u16)>,
	scroll_top: Cell<usize>,
	theme: SharedTheme,
	key_config: SharedKeyConfig,
}

impl CommitList {
	///
	pub fn new(
		title: &str,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			items: ItemBatch::default(),
			marked: Vec::with_capacity(2),
			selection: 0,
			branch: None,
			count_total: 0,
			scroll_state: (Instant::now(), 0_f32),
			tags: None,
			current_size: Cell::new((0, 0)),
			scroll_top: Cell::new(0),
			theme,
			key_config,
			title: title.into(),
		}
	}

	///
	pub fn items(&mut self) -> &mut ItemBatch {
		&mut self.items
	}

	///
	pub fn set_title(&mut self, title: &str) {
		self.title = title.into();
	}

	///
	pub fn set_branch(&mut self, name: Option<String>) {
		self.branch = name;
	}

	///
	pub const fn selection(&self) -> usize {
		self.selection
	}

	///
	pub fn current_size(&self) -> (u16, u16) {
		self.current_size.get()
	}

	///
	pub fn set_count_total(&mut self, total: usize) {
		self.count_total = total;
		self.selection =
			cmp::min(self.selection, self.selection_max());
	}

	///
	#[allow(clippy::missing_const_for_fn)]
	pub fn selection_max(&self) -> usize {
		self.count_total.saturating_sub(1)
	}

	///
	pub const fn tags(&self) -> Option<&Tags> {
		self.tags.as_ref()
	}

	///
	pub fn clear(&mut self) {
		self.items.clear();
	}

	///
	pub fn set_tags(&mut self, tags: Tags) {
		self.tags = Some(tags);
	}

	///
	pub fn selected_entry(&self) -> Option<&LogEntry> {
		self.items.iter().nth(
			self.selection.saturating_sub(self.items.index_offset()),
		)
	}

	///
	pub fn selected_entry_marked(&self) -> bool {
		self.selected_entry()
			.and_then(|e| self.is_marked(&e.id))
			.unwrap_or_default()
	}

	///
	pub fn marked_count(&self) -> usize {
		self.marked.len()
	}

	///
	pub fn marked(&self) -> &[CommitId] {
		&self.marked
	}

	pub fn copy_entry_hash(&self) -> Result<()> {
		if let Some(e) = self.items.iter().nth(
			self.selection.saturating_sub(self.items.index_offset()),
		) {
			crate::clipboard::copy_string(&e.hash_short)?;
		}
		Ok(())
	}

	fn move_selection(&mut self, scroll: ScrollType) -> Result<bool> {
		self.update_scroll_speed();

		#[allow(clippy::cast_possible_truncation)]
		let speed_int = usize::try_from(self.scroll_state.1 as i64)?.max(1);

		let page_offset =
			usize::from(self.current_size.get().1).saturating_sub(1);

		let new_selection = match scroll {
			ScrollType::Up => {
				self.selection.saturating_sub(speed_int)
			}
			ScrollType::Down => {
				self.selection.saturating_add(speed_int)
			}
			ScrollType::PageUp => {
				self.selection.saturating_sub(page_offset)
			}
			ScrollType::PageDown => {
				self.selection.saturating_add(page_offset)
			}
			ScrollType::Home => 0,
			ScrollType::End => self.selection_max(),
		};

		let new_selection =
			cmp::min(new_selection, self.selection_max());

		let needs_update = new_selection != self.selection;

		self.selection = new_selection;

		Ok(needs_update)
	}

	fn mark(&mut self) {
		if let Some(e) = self.selected_entry() {
			let id = e.id;
			if self.is_marked(&id).unwrap_or_default() {
				self.marked.retain(|marked| marked != &id);
			} else {
				self.marked.push(id);
			}
		}
	}

	fn update_scroll_speed(&mut self) {
		const REPEATED_SCROLL_THRESHOLD_MILLIS: u128 = 300;
		const SCROLL_SPEED_START: f32 = 0.1_f32;
		const SCROLL_SPEED_MAX: f32 = 10_f32;
		const SCROLL_SPEED_MULTIPLIER: f32 = 1.05_f32;

		let now = Instant::now();

		let since_last_scroll =
			now.duration_since(self.scroll_state.0);

		self.scroll_state.0 = now;

		let speed = if since_last_scroll.as_millis()
			< REPEATED_SCROLL_THRESHOLD_MILLIS
		{
			self.scroll_state.1 * SCROLL_SPEED_MULTIPLIER
		} else {
			SCROLL_SPEED_START
		};

		self.scroll_state.1 = speed.min(SCROLL_SPEED_MAX);
	}

	fn is_marked(&self, id: &CommitId) -> Option<bool> {
		if self.marked.is_empty() {
			None
		} else {
			let found = self.marked.iter().any(|entry| entry == id);
			Some(found)
		}
	}

	fn get_entry_to_add<'a>(
		e: &'a LogEntry,
		selected: bool,
		tags: Option<String>,
		theme: &Theme,
		width: usize,
		now: DateTime<Local>,
		marked: Option<bool>,
	) -> Spans<'a> {
		let mut txt: Vec<Span> = Vec::with_capacity(
			ELEMENTS_PER_LINE + if marked.is_some() { 2 } else { 0 },
		);

		let splitter_txt = Cow::from(symbol::EMPTY_SPACE);
		let splitter =
			Span::styled(splitter_txt, theme.text(true, selected));

		// marker
		if let Some(marked) = marked {
			txt.push(Span::styled(
				Cow::from(if marked {
					symbol::CHECKMARK
				} else {
					symbol::EMPTY_SPACE
				}),
				theme.log_marker(selected),
			));
			txt.push(splitter.clone());
		}

		// commit hash
		txt.push(Span::styled(
			Cow::from(&*e.hash_short),
			theme.commit_hash(selected),
		));

		txt.push(splitter.clone());

		// commit timestamp
		txt.push(Span::styled(
			Cow::from(e.time_to_string(now)),
			theme.commit_time(selected),
		));

		txt.push(splitter.clone());

		let author_width =
			(width.saturating_sub(19) / 3).max(3).min(20);
		let author = string_width_align(&e.author, author_width);

		// commit author
		txt.push(Span::styled::<String>(
			author,
			theme.commit_author(selected),
		));

		txt.push(splitter.clone());

		// commit tags
		txt.push(Span::styled(
			Cow::from(if let Some(tags) = tags {
				format!(" {}", tags)
			} else {
				String::from("")
			}),
			theme.tags(selected),
		));

		txt.push(splitter);

		// commit msg
		txt.push(Span::styled(
			Cow::from(&*e.msg),
			theme.text(true, selected),
		));

		Spans::from(txt)
	}

	fn get_text(&self, height: usize, width: usize) -> Vec<Spans> {
		let selection = self.relative_selection();

		let mut txt: Vec<Spans> = Vec::with_capacity(height);

		let now = Local::now();

		let any_marked = !self.marked.is_empty();

		for (idx, e) in self
			.items
			.iter()
			.skip(self.scroll_top.get())
			.take(height)
			.enumerate()
		{
			let tags = self
				.tags
				.as_ref()
				.and_then(|t| t.get(&e.id))
				.map(|tags| tags.join(" "));

			let marked = if any_marked {
				self.is_marked(&e.id)
			} else {
				None
			};

			txt.push(Self::get_entry_to_add(
				e,
				idx + self.scroll_top.get() == selection,
				tags,
				&self.theme,
				width,
				now,
				marked,
			));
		}

		txt
	}

	#[allow(clippy::missing_const_for_fn)]
	fn relative_selection(&self) -> usize {
		self.selection.saturating_sub(self.items.index_offset())
	}

	pub fn select_entry(&mut self, position: usize) {
		self.selection = position;
	}
}

impl DrawableComponent for CommitList {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		let current_size = (
			area.width.saturating_sub(2),
			area.height.saturating_sub(2),
		);
		self.current_size.set(current_size);

		let height_in_lines = self.current_size.get().1 as usize;
		let selection = self.relative_selection();

		self.scroll_top.set(calc_scroll_top(
			self.scroll_top.get(),
			height_in_lines,
			selection,
		));

		let branch_post_fix =
			self.branch.as_ref().map(|b| format!("- {{{}}}", b));

		let title = format!(
			"{} {}/{} {}",
			self.title,
			self.count_total.saturating_sub(self.selection),
			self.count_total,
			branch_post_fix.as_deref().unwrap_or(""),
		);

		f.render_widget(
			Paragraph::new(
				self.get_text(
					height_in_lines,
					current_size.0 as usize,
				),
			)
			.block(
				Block::default()
					.borders(Borders::ALL)
					.title(Span::styled(
						title.as_str(),
						self.theme.title(true),
					))
					.border_style(self.theme.block(true)),
			)
			.alignment(Alignment::Left),
			area,
		);

		ui::draw_scrollbar(
			f,
			area,
			&self.theme,
			self.count_total
				.saturating_sub(height_in_lines.saturating_sub(2)),
			self.selection,
		);

		Ok(())
	}
}

impl Component for CommitList {
	fn event(&mut self, ev: Event) -> Result<EventState> {
		if let Event::Key(k) = ev {
			let selection_changed = if k == self.key_config.move_up {
				self.move_selection(ScrollType::Up)?
			} else if k == self.key_config.move_down {
				self.move_selection(ScrollType::Down)?
			} else if k == self.key_config.shift_up
				|| k == self.key_config.home
			{
				self.move_selection(ScrollType::Home)?
			} else if k == self.key_config.shift_down
				|| k == self.key_config.end
			{
				self.move_selection(ScrollType::End)?
			} else if k == self.key_config.page_up {
				self.move_selection(ScrollType::PageUp)?
			} else if k == self.key_config.page_down {
				self.move_selection(ScrollType::PageDown)?
			} else if k == self.key_config.log_mark_commit {
				self.mark();
				true
			} else {
				false
			};
			return Ok(selection_changed.into());
		}

		Ok(EventState::NotConsumed)
	}

	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		_force_all: bool,
	) -> CommandBlocking {
		out.push(CommandInfo::new(
			strings::commands::scroll(&self.key_config),
			self.selected_entry().is_some(),
			true,
		));
		out.push(CommandInfo::new(
			strings::commands::commit_list_mark(
				&self.key_config,
				self.selected_entry_marked(),
			),
			true,
			true,
		));
		CommandBlocking::PassingOn
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_string_width_align() {
		assert_eq!(string_width_align("123", 3), "123");
		assert_eq!(string_width_align("123", 2), "..");
		assert_eq!(string_width_align("123", 3), "123");
		assert_eq!(string_width_align("12345", 6), "12345 ");
		assert_eq!(string_width_align("1234556", 4), "12..");
	}

	#[test]
	fn test_string_width_align_unicode() {
		assert_eq!(string_width_align("äste", 3), "ä..");
		assert_eq!(
			string_width_align("wüsten äste", 10),
			"wüsten ä.."
		);
		assert_eq!(
			string_width_align("Jon Grythe Stødle", 19),
			"Jon Grythe Stødle  "
		);
	}
}

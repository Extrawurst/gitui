use super::{
	utils::scroll_vertical::VerticalScroll, CommandBlocking,
	CommandInfo, Component, DrawableComponent, EventState,
	SyntaxTextComponent,
};
use crate::{
	keys::SharedKeyConfig,
	queue::{InternalEvent, Queue},
	strings::{self, order},
	ui::{self, common_nav, style::SharedTheme},
	AsyncAppNotification, AsyncNotification,
};
use anyhow::Result;
use asyncgit::{
	sync::{self, CommitId, TreeFile},
	CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use filetreelist::{FileTree, FileTreeItem};
use std::{collections::BTreeSet, convert::From, path::Path};
use tui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	text::Span,
	widgets::{Block, Borders},
	Frame,
};

const FOLDER_ICON_COLLAPSED: &str = "\u{25b8}"; //▸
const FOLDER_ICON_EXPANDED: &str = "\u{25be}"; //▾
const EMPTY_STR: &str = "";

enum Focus {
	Tree,
	File,
}

pub struct RevisionFilesComponent {
	queue: Queue,
	theme: SharedTheme,
	//TODO: store TreeFiles in `tree`
	files: Vec<TreeFile>,
	current_file: SyntaxTextComponent,
	tree: FileTree,
	scroll: VerticalScroll,
	revision: Option<CommitId>,
	focus: Focus,
	key_config: SharedKeyConfig,
}

impl RevisionFilesComponent {
	///
	pub fn new(
		queue: &Queue,
		sender: &Sender<AsyncAppNotification>,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			queue: queue.clone(),
			tree: FileTree::default(),
			scroll: VerticalScroll::new(),
			current_file: SyntaxTextComponent::new(
				sender,
				key_config.clone(),
				theme.clone(),
			),
			theme,
			files: Vec::new(),
			revision: None,
			focus: Focus::Tree,
			key_config,
		}
	}

	///
	pub fn set_commit(&mut self, commit: CommitId) -> Result<()> {
		let same_id =
			self.revision.map(|c| c == commit).unwrap_or_default();
		if !same_id {
			self.files = sync::tree_files(CWD, commit)?;
			let filenames: Vec<&Path> =
				self.files.iter().map(|f| f.path.as_path()).collect();
			self.tree = FileTree::new(&filenames, &BTreeSet::new())?;
			self.tree.collapse_but_root();
			self.revision = Some(commit);
		}

		Ok(())
	}

	///
	pub fn update(&mut self, ev: AsyncNotification) {
		self.current_file.update(ev);
	}

	///
	pub fn any_work_pending(&self) -> bool {
		self.current_file.any_work_pending()
	}

	fn tree_item_to_span<'a>(
		item: &'a FileTreeItem,
		theme: &SharedTheme,
		selected: bool,
	) -> Span<'a> {
		let path = item.info().path_str();
		let indent = item.info().indent();

		let indent_str = if indent == 0 {
			String::from("")
		} else {
			format!("{:w$}", " ", w = (indent as usize) * 2)
		};

		let is_path = item.kind().is_path();
		let path_arrow = if is_path {
			if item.kind().is_path_collapsed() {
				FOLDER_ICON_COLLAPSED
			} else {
				FOLDER_ICON_EXPANDED
			}
		} else {
			EMPTY_STR
		};

		let path = format!("{}{}{}", indent_str, path_arrow, path);
		Span::styled(path, theme.file_tree_item(is_path, selected))
	}

	fn blame(&self) -> bool {
		self.selected_file_path().map_or(false, |path| {
			self.queue.push(InternalEvent::BlameFile(path));

			true
		})
	}

	fn file_history(&self) -> bool {
		self.selected_file_path().map_or(false, |path| {
			self.queue.push(InternalEvent::OpenFileRevlog(path));

			true
		})
	}

	fn selected_file_path(&self) -> Option<String> {
		self.tree.selected_file().map(|file| {
			file.full_path_str()
				.strip_prefix("./")
				.unwrap_or_default()
				.to_string()
		})
	}

	fn selection_changed(&mut self) {
		//TODO: retrieve TreeFile from tree datastructure
		if let Some(file) = self
			.tree
			.selected_file()
			.map(|file| file.full_path_str().to_string())
		{
			let path = Path::new(&file);
			if let Some(item) =
				self.files.iter().find(|f| f.path == path)
			{
				if let Ok(path) = path.strip_prefix("./") {
					return self.current_file.load_file(
						path.to_string_lossy().to_string(),
						item,
					);
				}
			}
			self.current_file.clear();
		}
	}

	fn draw_tree<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
		let tree_height = usize::from(area.height.saturating_sub(2));

		self.tree.visual_selection().map_or_else(
			|| {
				self.scroll.reset();
			},
			|selection| {
				self.scroll.update(
					selection.index,
					selection.count,
					tree_height,
				);
			},
		);

		let items = self
			.tree
			.iterate(self.scroll.get_top(), tree_height)
			.map(|(item, selected)| {
				Self::tree_item_to_span(item, &self.theme, selected)
			});

		let is_tree_focused = matches!(self.focus, Focus::Tree);

		let title = format!(
			"Files at [{}]",
			self.revision
				.map(|c| c.get_short_string())
				.unwrap_or_default()
		);
		ui::draw_list_block(
			f,
			area,
			Block::default()
				.title(Span::styled(
					title,
					self.theme.title(is_tree_focused),
				))
				.borders(Borders::ALL)
				.border_style(self.theme.block(is_tree_focused)),
			items,
		);

		if is_tree_focused {
			self.scroll.draw(f, area, &self.theme);
		}
	}
}

impl DrawableComponent for RevisionFilesComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.is_visible() {
			let chunks = Layout::default()
				.direction(Direction::Horizontal)
				.constraints(
					[
						Constraint::Percentage(40),
						Constraint::Percentage(60),
					]
					.as_ref(),
				)
				.split(area);

			self.draw_tree(f, chunks[0]);

			self.current_file.draw(f, chunks[1])?;
		}
		Ok(())
	}
}

impl Component for RevisionFilesComponent {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if matches!(self.focus, Focus::Tree) || force_all {
			out.push(
				CommandInfo::new(
					strings::commands::blame_file(&self.key_config),
					self.tree.selected_file().is_some(),
					true,
				)
				.order(order::NAV),
			);
			out.push(
				CommandInfo::new(
					strings::commands::open_file_history(
						&self.key_config,
					),
					self.tree.selected_file().is_some(),
					true,
				)
				.order(order::RARE_ACTION),
			);
			tree_nav_cmds(&self.tree, &self.key_config, out);
		} else {
			self.current_file.commands(out, force_all);
		}

		CommandBlocking::PassingOn
	}

	fn event(
		&mut self,
		event: crossterm::event::Event,
	) -> Result<EventState> {
		if let Event::Key(key) = event {
			let is_tree_focused = matches!(self.focus, Focus::Tree);
			if is_tree_focused
				&& tree_nav(&mut self.tree, &self.key_config, key)
			{
				self.selection_changed();
				return Ok(EventState::Consumed);
			} else if key == self.key_config.blame {
				if self.blame() {
					self.hide();
					return Ok(EventState::Consumed);
				}
			} else if key == self.key_config.file_history {
				if self.file_history() {
					self.hide();
					return Ok(EventState::Consumed);
				}
			} else if key == self.key_config.move_right {
				if is_tree_focused {
					self.focus = Focus::File;
					self.current_file.focus(true);
					self.focus(true);
					return Ok(EventState::Consumed);
				}
			} else if key == self.key_config.move_left {
				if !is_tree_focused {
					self.focus = Focus::Tree;
					self.current_file.focus(false);
					self.focus(false);
					return Ok(EventState::Consumed);
				}
			} else if !is_tree_focused {
				return self.current_file.event(event);
			}
		}

		Ok(EventState::NotConsumed)
	}
}

//TODO: reuse for other tree usages
fn tree_nav_cmds(
	tree: &FileTree,
	key_config: &SharedKeyConfig,
	out: &mut Vec<CommandInfo>,
) {
	out.push(
		CommandInfo::new(
			strings::commands::navigate_tree(key_config),
			!tree.is_empty(),
			true,
		)
		.order(order::NAV),
	);
}

//TODO: reuse for other tree usages
fn tree_nav(
	tree: &mut FileTree,
	key_config: &SharedKeyConfig,
	key: crossterm::event::KeyEvent,
) -> bool {
	if let Some(common_nav) = common_nav(key, key_config) {
		tree.move_selection(common_nav)
	} else if key == key_config.tree_collapse_recursive {
		tree.collapse_recursive();
		true
	} else if key == key_config.tree_expand_recursive {
		tree.expand_recursive();
		true
	} else {
		false
	}
}

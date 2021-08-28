use super::visibility_blocking;
use crate::{
	components::{
		CommandBlocking, CommandInfo, CommitList, Component,
		DrawableComponent, EventState,
	},
	keys::SharedKeyConfig,
	queue::{InternalEvent, Queue},
	strings,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::{
	sync::{diff_contains_file, get_commits_info, CommitId},
	AsyncGitNotification, AsyncLog, FetchStatus, CWD,
};
use crossbeam_channel::Sender;
use crossterm::event::Event;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};

const SLICE_SIZE: usize = 1200;

///
pub struct FileRevlogComponent {
	list: CommitList,
	git_log: Option<AsyncLog>,
	queue: Queue,
	sender: Sender<AsyncGitNotification>,
	visible: bool,
	file_path: Option<String>,
	key_config: SharedKeyConfig,
}

impl FileRevlogComponent {
	///
	pub fn new(
		queue: &Queue,
		sender: &Sender<AsyncGitNotification>,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			queue: queue.clone(),
			sender: sender.clone(),
			list: CommitList::new("", theme, key_config.clone()),
			git_log: None,
			visible: false,
			file_path: None,
			key_config,
		}
	}

	///
	pub fn open(&mut self, file_path: &str) -> Result<()> {
		self.file_path = Some(file_path.into());
		self.list.set_title(&strings::file_log_title(
			&self.key_config,
			file_path,
		));

		let filter = diff_contains_file(file_path.into());
		self.git_log =
			Some(AsyncLog::new(&self.sender, Some(filter)));
		self.show()?;

		self.update()?;

		Ok(())
	}

	///
	pub fn any_work_pending(&self) -> bool {
		self.git_log.as_ref().map_or(false, AsyncLog::is_pending)
	}

	///
	pub fn update(&mut self) -> Result<()> {
		if let Some(ref mut git_log) = self.git_log {
			let log_changed =
				git_log.fetch()? == FetchStatus::Started;

			self.list.set_count_total(git_log.count()?);

			let selection = self.list.selection();
			let selection_max = self.list.selection_max();
			if self.list.items().needs_data(selection, selection_max)
				|| log_changed
			{
				self.fetch_commits()?;
			}
		}

		Ok(())
	}

	///
	pub fn update_git(
		&mut self,
		event: AsyncGitNotification,
	) -> Result<()> {
		if self.visible {
			match event {
				AsyncGitNotification::CommitFiles
				| AsyncGitNotification::Log => self.update()?,
				_ => (),
			}
		}

		Ok(())
	}

	fn fetch_commits(&mut self) -> Result<()> {
		if let Some(git_log) = &self.git_log {
			let want_min =
				self.list.selection().saturating_sub(SLICE_SIZE / 2);

			let commits = get_commits_info(
				CWD,
				&git_log.get_slice(want_min, SLICE_SIZE)?,
				self.list.current_size().0.into(),
			);

			if let Ok(commits) = commits {
				self.list.items().set_items(want_min, commits);
			}
		}

		Ok(())
	}

	fn selected_commit(&self) -> Option<CommitId> {
		self.list.selected_entry().map(|e| e.id)
	}
}

impl DrawableComponent for FileRevlogComponent {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		area: Rect,
	) -> Result<()> {
		if self.visible {
			f.render_widget(Clear, area);
			self.list.draw(f, area)?;
		}

		Ok(())
	}
}

impl Component for FileRevlogComponent {
	fn event(&mut self, event: Event) -> Result<EventState> {
		if self.is_visible() {
			let event_used = self.list.event(event)?;

			if event_used.is_consumed() {
				self.update()?;

				return Ok(EventState::Consumed);
			} else if let Event::Key(key) = event {
				if key == self.key_config.exit_popup {
					self.hide();
				} else if key == self.key_config.enter {
					self.hide();

					return self.selected_commit().map_or(
						Ok(EventState::NotConsumed),
						|id| {
							self.queue.push(
								InternalEvent::InspectCommit(
									id, None,
								),
							);
							Ok(EventState::Consumed)
						},
					);
				}

				return Ok(EventState::Consumed);
			}
		}

		Ok(EventState::NotConsumed)
	}

	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			out.push(
				CommandInfo::new(
					strings::commands::close_popup(&self.key_config),
					true,
					true,
				)
				.order(1),
			);
			out.push(
				CommandInfo::new(
					strings::commands::log_details_toggle(
						&self.key_config,
					),
					true,
					self.selected_commit().is_some(),
				)
				.order(1),
			);
		}

		visibility_blocking(self)
	}

	fn is_visible(&self) -> bool {
		self.visible
	}

	fn hide(&mut self) {
		self.visible = false;
	}

	fn show(&mut self) -> Result<()> {
		self.visible = true;

		Ok(())
	}
}

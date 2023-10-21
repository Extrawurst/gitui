use super::{
	textinput::TextInputComponent, visibility_blocking,
	CommandBlocking, CommandInfo, Component, DrawableComponent,
	EventState,
};
use crate::{
	keys::{key_match, SharedKeyConfig},
	queue::{AppTabs, InternalEvent, Queue},
	strings,
	tabs::StashingOptions,
	ui::style::SharedTheme,
};
use anyhow::Result;
use asyncgit::sync::{self, RepoPathRef};
use crossterm::event::Event;
use ratatui::{backend::Backend, layout::Rect, Frame};

pub struct StashMsgComponent<'a> {
	repo: RepoPathRef,
	options: StashingOptions,
	input: TextInputComponent<'a>,
	queue: Queue,
	key_config: SharedKeyConfig,
}

impl<'a> DrawableComponent for StashMsgComponent<'a> {
	fn draw<B: Backend>(
		&self,
		f: &mut Frame<B>,
		rect: Rect,
	) -> Result<()> {
		self.input.draw(f, rect)?;

		Ok(())
	}
}

impl<'a> Component for StashMsgComponent<'a> {
	fn commands(
		&self,
		out: &mut Vec<CommandInfo>,
		force_all: bool,
	) -> CommandBlocking {
		if self.is_visible() || force_all {
			self.input.commands(out, force_all);

			out.push(CommandInfo::new(
				strings::commands::stashing_confirm_msg(
					&self.key_config,
				),
				true,
				true,
			));
		}

		visibility_blocking(self)
	}

	fn event(&mut self, ev: &Event) -> Result<EventState> {
		if self.is_visible() {
			if self.input.event(ev)?.is_consumed() {
				return Ok(EventState::Consumed);
			}
			let input_text = self.input.get_text();
			if let Event::Key(e) = ev {
				if key_match(e, self.key_config.keys.enter) {
					let result = sync::stash_save(
						&self.repo.borrow(),
						if input_text.is_empty() {
							None
						} else {
							Some(input_text.as_str())
						},
						self.options.stash_untracked,
						self.options.keep_index,
					);
					match result {
						Ok(_) => {
							self.input.clear();
							self.hide();

							self.queue.push(
								InternalEvent::TabSwitch(
									AppTabs::Stashlist,
								),
							);
						}
						Err(e) => {
							self.hide();
							log::error!(
								"e: {} (options: {:?})",
								e,
								self.options
							);
							self.queue.push(
                                InternalEvent::ShowErrorMsg(format!(
                                    "stash error:\n{}\noptions:\n{:?}",
                                    e, self.options
                                )),
                            );
						}
					}
				}

				// stop key event propagation
				return Ok(EventState::Consumed);
			}
		}
		Ok(EventState::NotConsumed)
	}

	fn is_visible(&self) -> bool {
		self.input.is_visible()
	}

	fn hide(&mut self) {
		self.input.hide();
	}

	fn show(&mut self) -> Result<()> {
		self.input.show()?;

		Ok(())
	}
}

impl<'a> StashMsgComponent<'a> {
	///
	pub fn new(
		repo: RepoPathRef,
		queue: Queue,
		theme: SharedTheme,
		key_config: SharedKeyConfig,
	) -> Self {
		Self {
			options: StashingOptions::default(),
			queue,
			input: TextInputComponent::new(
				theme,
				key_config.clone(),
				&strings::stash_popup_title(&key_config),
				&strings::stash_popup_msg(&key_config),
				true,
			)
			.with_input_type(super::InputType::Singleline),
			key_config,
			repo,
		}
	}

	///
	pub fn options(&mut self, options: StashingOptions) {
		self.options = options;
	}
}

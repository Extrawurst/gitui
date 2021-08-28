use crate::{components::AppOption, tabs::StashingOptions};
use asyncgit::sync::{diff::DiffLinePosition, CommitId, CommitTags};
use bitflags::bitflags;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

bitflags! {
	/// flags defining what part of the app need to update
	pub struct NeedsUpdate: u32 {
		/// app::update
		const ALL = 0b001;
		/// diff may have changed (app::update_diff)
		const DIFF = 0b010;
		/// commands might need updating (app::update_commands)
		const COMMANDS = 0b100;
		/// branches have changed
		const BRANCHES = 0b1000;
	}
}

/// data of item that is supposed to be reset
pub struct ResetItem {
	/// path to the item (folder/file)
	pub path: String,
	/// are talking about a folder here? otherwise it's a single file
	pub is_folder: bool,
}

///
pub enum Action {
	Reset(ResetItem),
	ResetHunk(String, u64),
	ResetLines(String, Vec<DiffLinePosition>),
	StashDrop(Vec<CommitId>),
	StashPop(CommitId),
	DeleteBranch(String, bool),
	DeleteTag(String),
	ForcePush(String, bool),
	PullMerge { incoming: usize, rebase: bool },
	AbortMerge,
}

///
pub enum InternalEvent {
	///
	ConfirmAction(Action),
	///
	ConfirmedAction(Action),
	///
	ShowErrorMsg(String),
	///
	Update(NeedsUpdate),
	///
	StatusLastFileMoved,
	/// open commit msg input
	OpenCommit,
	///
	PopupStashing(StashingOptions),
	///
	TabSwitch,
	///
	InspectCommit(CommitId, Option<CommitTags>),
	///
	CompareCommits(CommitId, Option<CommitId>),
	///
	SelectCommitInRevlog(CommitId),
	///
	TagCommit(CommitId),
	///
	Tags,
	///
	BlameFile(String),
	///
	OpenFileRevlog(String),
	///
	CreateBranch,
	///
	RenameBranch(String, String),
	///
	SelectBranch,
	///
	OpenExternalEditor(Option<String>),
	///
	Push(String, bool, bool),
	///
	Pull(String),
	///
	PushTags,
	///
	OpenFileTree(CommitId),
	///
	OptionSwitched(AppOption),
}

/// single threaded simple queue for components to communicate with each other
#[derive(Clone)]
pub struct Queue {
	data: Rc<RefCell<VecDeque<InternalEvent>>>,
}

impl Queue {
	pub fn new() -> Self {
		Self {
			data: Rc::new(RefCell::new(VecDeque::new())),
		}
	}

	pub fn push(&self, ev: InternalEvent) {
		self.data.borrow_mut().push_back(ev);
	}

	pub fn pop(&self) -> Option<InternalEvent> {
		self.data.borrow_mut().pop_front()
	}

	pub fn clear(&self) {
		self.data.borrow_mut().clear();
	}
}

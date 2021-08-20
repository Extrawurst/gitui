use crate::{
	error::Result,
	sync::{self, CommitId},
	AsyncGitNotification, StatusItem, CWD,
};
use crossbeam_channel::Sender;
use std::sync::{
	atomic::{AtomicUsize, Ordering},
	Arc, Mutex,
};

type ResultType = Vec<StatusItem>;
struct Request<R, A>(R, A);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CommitFilesParams {
	id: CommitId,
	other: Option<CommitId>,
}

///
pub struct AsyncCommitFiles {
	current:
		Arc<Mutex<Option<Request<CommitFilesParams, ResultType>>>>,
	sender: Sender<AsyncGitNotification>,
	pending: Arc<AtomicUsize>,
}

impl AsyncCommitFiles {
	///
	pub fn new(sender: &Sender<AsyncGitNotification>) -> Self {
		Self {
			current: Arc::new(Mutex::new(None)),
			sender: sender.clone(),
			pending: Arc::new(AtomicUsize::new(0)),
		}
	}

	///
	pub fn current(
		&mut self,
	) -> Result<Option<(CommitFilesParams, ResultType)>> {
		let c = self.current.lock()?;

		c.as_ref()
			.map_or(Ok(None), |c| Ok(Some((c.0, c.1.clone()))))
	}

	///
	pub fn is_pending(&self) -> bool {
		self.pending.load(Ordering::Relaxed) > 0
	}

	///
	pub fn fetch(&mut self, params: CommitFilesParams) -> Result<()> {
		if self.is_pending() {
			return Ok(());
		}

		log::trace!("request: {:?}", params);

		{
			let current = self.current.lock()?;
			if let Some(c) = &*current {
				if c.0 == params {
					return Ok(());
				}
			}
		}

		let arc_current = Arc::clone(&self.current);
		let sender = self.sender.clone();
		let arc_pending = Arc::clone(&self.pending);

		self.pending.fetch_add(1, Ordering::Relaxed);

		rayon_core::spawn(move || {
			Self::fetch_helper(params, &arc_current)
				.expect("failed to fetch");

			arc_pending.fetch_sub(1, Ordering::Relaxed);

			sender
				.send(AsyncGitNotification::CommitFiles)
				.expect("error sending");
		});

		Ok(())
	}

	fn fetch_helper(
		params: CommitFilesParams,
		arc_current: &Arc<
			Mutex<Option<Request<CommitFilesParams, ResultType>>>,
		>,
	) -> Result<()> {
		let res =
			sync::get_commit_files(CWD, params.id, params.other)?;

		log::trace!("get_commit_files: {:?} ({})", params, res.len());

		{
			let mut current = arc_current.lock()?;
			*current = Some(Request(params, res));
		}

		Ok(())
	}
}

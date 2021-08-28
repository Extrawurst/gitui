use super::CommitId;
use crate::{error::Result, sync::commit_files::get_commit_diff};
use git2::{Commit, Oid, Repository};
use std::{
	cmp::Ordering,
	collections::{BinaryHeap, HashSet},
	sync::Arc,
};

struct TimeOrderedCommit<'a>(Commit<'a>);

impl<'a> Eq for TimeOrderedCommit<'a> {}

impl<'a> PartialEq for TimeOrderedCommit<'a> {
	fn eq(&self, other: &Self) -> bool {
		self.0.time().eq(&other.0.time())
	}
}

impl<'a> PartialOrd for TimeOrderedCommit<'a> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.0.time().partial_cmp(&other.0.time())
	}
}

impl<'a> Ord for TimeOrderedCommit<'a> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.0.time().cmp(&other.0.time())
	}
}

///
pub type LogWalkerFilter = Arc<
	Box<dyn Fn(&Repository, &CommitId) -> Result<bool> + Send + Sync>,
>;

///
pub fn diff_contains_file(file_path: String) -> LogWalkerFilter {
	Arc::new(Box::new(
		move |repo: &Repository,
		      commit_id: &CommitId|
		      -> Result<bool> {
			let diff = get_commit_diff(
				repo,
				*commit_id,
				Some(file_path.clone()),
			)?;

			let contains_file = diff.deltas().len() > 0;

			Ok(contains_file)
		},
	))
}

///
pub struct LogWalker<'a> {
	commits: BinaryHeap<TimeOrderedCommit<'a>>,
	visited: HashSet<Oid>,
	limit: usize,
	repo: &'a Repository,
	filter: Option<LogWalkerFilter>,
}

impl<'a> LogWalker<'a> {
	///
	pub fn new(repo: &'a Repository, limit: usize) -> Result<Self> {
		let c = repo.head()?.peel_to_commit()?;

		let mut commits = BinaryHeap::with_capacity(10);
		commits.push(TimeOrderedCommit(c));

		Ok(Self {
			commits,
			limit,
			visited: HashSet::with_capacity(1000),
			repo,
			filter: None,
		})
	}

	///
	pub fn filter(self, filter: Option<LogWalkerFilter>) -> Self {
		Self { filter, ..self }
	}

	///
	pub fn read(&mut self, out: &mut Vec<CommitId>) -> Result<usize> {
		let mut count = 0_usize;

		while let Some(c) = self.commits.pop() {
			for p in c.0.parents() {
				self.visit(p);
			}

			let id: CommitId = c.0.id().into();
			let commit_should_be_included =
				if let Some(ref filter) = self.filter {
					filter(self.repo, &id)?
				} else {
					true
				};

			if commit_should_be_included {
				out.push(id);
			}

			count += 1;
			if count == self.limit {
				break;
			}
		}

		Ok(count)
	}

	//
	fn visit(&mut self, c: Commit<'a>) {
		if !self.visited.contains(&c.id()) {
			self.visited.insert(c.id());
			self.commits.push(TimeOrderedCommit(c));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::error::Result;
	use crate::sync::{
		commit, commit_files::get_commit_diff, get_commits_info,
		stage_add_file, tests::repo_init_empty,
	};
	use pretty_assertions::assert_eq;
	use std::{fs::File, io::Write, path::Path};

	#[test]
	fn test_limit() -> Result<()> {
		let file_path = Path::new("foo");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		File::create(&root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		commit(repo_path, "commit1").unwrap();
		File::create(&root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		let oid2 = commit(repo_path, "commit2").unwrap();

		let mut items = Vec::new();
		let mut walk = LogWalker::new(&repo, 1)?;
		walk.read(&mut items).unwrap();

		assert_eq!(items.len(), 1);
		assert_eq!(items[0], oid2.into());

		Ok(())
	}

	#[test]
	fn test_logwalker() -> Result<()> {
		let file_path = Path::new("foo");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		File::create(&root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		commit(repo_path, "commit1").unwrap();
		File::create(&root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();
		let oid2 = commit(repo_path, "commit2").unwrap();

		let mut items = Vec::new();
		let mut walk = LogWalker::new(&repo, 100)?;
		walk.read(&mut items).unwrap();

		let info = get_commits_info(repo_path, &items, 50).unwrap();
		dbg!(&info);

		assert_eq!(items.len(), 2);
		assert_eq!(items[0], oid2.into());

		let mut items = Vec::new();
		walk.read(&mut items).unwrap();

		assert_eq!(items.len(), 0);

		Ok(())
	}

	#[test]
	fn test_logwalker_with_filter() -> Result<()> {
		let file_path = Path::new("foo");
		let second_file_path = Path::new("baz");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		File::create(&root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();

		let _first_commit_id = commit(repo_path, "commit1").unwrap();

		File::create(&root.join(second_file_path))?
			.write_all(b"a")?;
		stage_add_file(repo_path, second_file_path).unwrap();

		let second_commit_id = commit(repo_path, "commit2").unwrap();

		File::create(&root.join(file_path))?.write_all(b"b")?;
		stage_add_file(repo_path, file_path).unwrap();

		let _third_commit_id = commit(repo_path, "commit3").unwrap();

		let diff_contains_baz = |repo: &Repository,
		                         commit_id: &CommitId|
		 -> Result<bool> {
			let diff = get_commit_diff(
				&repo,
				*commit_id,
				Some("baz".into()),
			)?;

			let contains_file = diff.deltas().len() > 0;

			Ok(contains_file)
		};

		let mut items = Vec::new();
		let mut walker = LogWalker::new(&repo, 100)?
			.filter(Some(Arc::new(Box::new(diff_contains_baz))));
		walker.read(&mut items).unwrap();

		assert_eq!(items.len(), 1);
		assert_eq!(items[0], second_commit_id.into());

		let mut items = Vec::new();
		walker.read(&mut items).unwrap();

		assert_eq!(items.len(), 0);

		let diff_contains_bar = diff_contains_file("bar".into());

		let mut items = Vec::new();
		let mut walker = LogWalker::new(&repo, 100)?
			.filter(Some(diff_contains_bar));
		walker.read(&mut items).unwrap();

		assert_eq!(items.len(), 0);

		Ok(())
	}
}

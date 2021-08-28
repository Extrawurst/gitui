//! Functions for getting infos about files in commits

use super::{stash::is_stash_commit, utils::repo, CommitId};
use crate::{
	error::Error, error::Result, StatusItem, StatusItemType,
};
use git2::{Diff, DiffDelta, DiffOptions, Repository};
use scopetime::scope_time;
use std::cmp::Ordering;

/// get all files that are part of a commit
pub fn get_commit_files(
	repo_path: &str,
	id: CommitId,
	other: Option<CommitId>,
) -> Result<Vec<StatusItem>> {
	scope_time!("get_commit_files");

	let repo = repo(repo_path)?;

	let diff = if let Some(other) = other {
		get_compare_commits_diff(&repo, (id, other), None)?
	} else {
		get_commit_diff(&repo, id, None)?
	};

	let mut res = Vec::new();

	diff.foreach(
		&mut |delta: DiffDelta<'_>, _progress| {
			res.push(StatusItem {
				path: delta
					.new_file()
					.path()
					.map(|p| p.to_str().unwrap_or("").to_string())
					.unwrap_or_default(),
				status: StatusItemType::from(delta.status()),
			});
			true
		},
		None,
		None,
		None,
	)?;

	Ok(res)
}

/// get diff of two arbitrary commits
#[allow(clippy::needless_pass_by_value)]
pub fn get_compare_commits_diff(
	repo: &Repository,
	ids: (CommitId, CommitId),
	pathspec: Option<String>,
) -> Result<Diff<'_>> {
	// scope_time!("get_compare_commits_diff");

	let commits = (
		repo.find_commit(ids.0.into())?,
		repo.find_commit(ids.1.into())?,
	);

	let commits = if commits.0.time().cmp(&commits.1.time())
		== Ordering::Greater
	{
		(commits.1, commits.0)
	} else {
		commits
	};

	let trees = (commits.0.tree()?, commits.1.tree()?);

	let mut opts = DiffOptions::new();
	if let Some(p) = &pathspec {
		opts.pathspec(p.clone());
	}
	opts.show_binary(true);

	let diff = repo.diff_tree_to_tree(
		Some(&trees.0),
		Some(&trees.1),
		Some(&mut opts),
	)?;

	Ok(diff)
}

/// get diff of a commit to its first parent
pub fn get_commit_diff(
	repo: &Repository,
	id: CommitId,
	pathspec: Option<String>,
) -> Result<Diff<'_>> {
	scope_time!("get_commit_diff");

	let commit = repo.find_commit(id.into())?;
	let commit_tree = commit.tree()?;
	let parent = if commit.parent_count() > 0 {
		repo.find_commit(commit.parent_id(0)?)
			.ok()
			.and_then(|c| c.tree().ok())
	} else {
		None
	};

	let mut opts = DiffOptions::new();
	if let Some(p) = &pathspec {
		opts.pathspec(p.clone());
	}
	opts.show_binary(true);

	let mut diff = repo.diff_tree_to_tree(
		parent.as_ref(),
		Some(&commit_tree),
		Some(&mut opts),
	)?;

	if is_stash_commit(
		repo.path().to_str().map_or_else(
			|| Err(Error::Generic("repo path utf8 err".to_owned())),
			Ok,
		)?,
		&id,
	)? {
		if let Ok(untracked_commit) = commit.parent_id(2) {
			let untracked_diff = get_commit_diff(
				repo,
				CommitId::new(untracked_commit),
				pathspec,
			)?;

			diff.merge(&untracked_diff)?;
		}
	}

	Ok(diff)
}

#[cfg(test)]
mod tests {
	use super::get_commit_files;
	use crate::{
		error::Result,
		sync::{
			commit, stage_add_file, stash_save,
			tests::{get_statuses, repo_init},
		},
		StatusItemType,
	};
	use std::{fs::File, io::Write, path::Path};

	#[test]
	fn test_smoke() -> Result<()> {
		let file_path = Path::new("file1.txt");
		let (_td, repo) = repo_init()?;
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		File::create(&root.join(file_path))?
			.write_all(b"test file1 content")?;

		stage_add_file(repo_path, file_path)?;

		let id = commit(repo_path, "commit msg")?;

		let diff = get_commit_files(repo_path, id, None)?;

		assert_eq!(diff.len(), 1);
		assert_eq!(diff[0].status, StatusItemType::New);

		Ok(())
	}

	#[test]
	fn test_stashed_untracked() -> Result<()> {
		let file_path = Path::new("file1.txt");
		let (_td, repo) = repo_init()?;
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		File::create(&root.join(file_path))?
			.write_all(b"test file1 content")?;

		let id = stash_save(repo_path, None, true, false)?;

		let diff = get_commit_files(repo_path, id, None)?;

		assert_eq!(diff.len(), 1);
		assert_eq!(diff[0].status, StatusItemType::New);

		Ok(())
	}

	#[test]
	fn test_stashed_untracked_and_modified() -> Result<()> {
		let file_path1 = Path::new("file1.txt");
		let file_path2 = Path::new("file2.txt");
		let (_td, repo) = repo_init()?;
		let root = repo.path().parent().unwrap();
		let repo_path = root.as_os_str().to_str().unwrap();

		File::create(&root.join(file_path1))?.write_all(b"test")?;
		stage_add_file(repo_path, file_path1)?;
		commit(repo_path, "c1")?;

		File::create(&root.join(file_path1))?
			.write_all(b"modified")?;
		File::create(&root.join(file_path2))?.write_all(b"new")?;

		assert_eq!(get_statuses(repo_path), (2, 0));

		let id = stash_save(repo_path, None, true, false)?;

		let diff = get_commit_files(repo_path, id, None)?;

		assert_eq!(diff.len(), 2);
		assert_eq!(diff[0].status, StatusItemType::Modified);
		assert_eq!(diff[1].status, StatusItemType::New);

		Ok(())
	}
}

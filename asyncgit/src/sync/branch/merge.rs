//! merging from upstream

use super::BranchType;
use crate::{
    error::{Error, Result},
    sync::utils,
};
use scopetime::scope_time;

///
pub fn branch_merge_upstream_fastforward(
    repo_path: &str,
    branch: &str,
) -> Result<()> {
    scope_time!("branch_merge_upstream");

    let repo = utils::repo(repo_path)?;

    let branch = repo.find_branch(branch, BranchType::Local)?;
    let upstream = branch.upstream()?;

    let upstream_commit =
        upstream.into_reference().peel_to_commit()?;

    let annotated =
        repo.find_annotated_commit(upstream_commit.id())?;

    let (analysis, _) = repo.merge_analysis(&[&annotated])?;

    if !analysis.is_fast_forward() {
        return Err(Error::Generic(
            "fast forward merge not possible".into(),
        ));
    }

    if analysis.is_unborn() {
        return Err(Error::Generic("head is unborn".into()));
    }

    repo.checkout_tree(upstream_commit.as_object(), None)?;

    repo.head()?.set_target(annotated.id(), "")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sync::{
        commit,
        remotes::{fetch_origin, push::push},
        stage_add_file,
        tests::{
            debug_cmd_print, get_commit_ids, repo_clone,
            repo_init_bare,
        },
        CommitId,
    };
    use git2::Repository;
    use std::{fs::File, io::Write, path::Path};

    // write, stage and commit a file
    fn write_commit_file(
        repo: &Repository,
        file: &str,
        content: &str,
        commit_name: &str,
    ) -> CommitId {
        File::create(
            repo.workdir().unwrap().join(file).to_str().unwrap(),
        )
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();

        stage_add_file(
            repo.workdir().unwrap().to_str().unwrap(),
            Path::new(file),
        )
        .unwrap();

        commit(repo.workdir().unwrap().to_str().unwrap(), commit_name)
            .unwrap()
    }

    #[test]
    fn test_merge() {
        let (r1_dir, _repo) = repo_init_bare().unwrap();

        let (clone1_dir, clone1) =
            repo_clone(r1_dir.path().to_str().unwrap()).unwrap();

        let (clone2_dir, clone2) =
            repo_clone(r1_dir.path().to_str().unwrap()).unwrap();

        // clone1

        let commit1 =
            write_commit_file(&clone1, "test.txt", "test", "commit1");

        push(
            clone1_dir.path().to_str().unwrap(),
            "origin",
            "master",
            false,
            None,
            None,
        )
        .unwrap();

        // clone2
        debug_cmd_print(
            clone2_dir.path().to_str().unwrap(),
            "git pull --ff",
        );

        let commit2 = write_commit_file(
            &clone2,
            "test2.txt",
            "test",
            "commit2",
        );

        push(
            clone2_dir.path().to_str().unwrap(),
            "origin",
            "master",
            false,
            None,
            None,
        )
        .unwrap();

        // clone1 again

        let bytes = fetch_origin(
            clone1_dir.path().to_str().unwrap(),
            "master",
            None,
            None,
        )
        .unwrap();
        assert!(bytes > 0);

        let bytes = fetch_origin(
            clone1_dir.path().to_str().unwrap(),
            "master",
            None,
            None,
        )
        .unwrap();
        assert_eq!(bytes, 0);

        branch_merge_upstream_fastforward(
            clone1_dir.path().to_str().unwrap(),
            "master",
        )
        .unwrap();

        let commits = get_commit_ids(&clone1, 10);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[1], commit1);
        assert_eq!(commits[0], commit2);
    }
}

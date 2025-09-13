use crate::{utils::{extract_namespace, extract_path_from_segment, insert_namespace_by_repo_path}, ImportContext};
use git2::{ObjectType, Oid, Repository};
use std::path::PathBuf;
use url::Url;

impl ImportContext {
    /// clone repo locally
    /// 1. Get mega url from postgres
    /// 2. Clone git repositories from mega, reserving the namespace as path where they are cloned
    pub(crate) async fn clone_a_repo_by_url(
        &mut self,
        clone_dir: &str,
        git_url_base: &str,
        git_url_suffix: &str,
    ) -> Result<PathBuf, git2::Error> {
        // mega_url = base + path
        //tracing::info!("enter clone_a_repo_by_url");
        let git_url = {
            let git_url_base = Url::parse(git_url_base)
                .unwrap_or_else(|_| panic!("Failed to parse mega url base: {}", &git_url_base));
            git_url_base
                .join(git_url_suffix)
                .expect("Failed to join url path")
        };
        tracing::info!("clone git_url:{}",git_url.clone());
        // namespace such as tokio-rs/tokio
        let namespace = extract_namespace(git_url.as_ref()).expect("Failed to parse URL");
        let behind_dir = extract_path_from_segment(git_url.as_ref(),"crates").expect("Failed to parse behind_dir");
        // The path the repo will be cloned into
        let path = PathBuf::from(clone_dir).join(behind_dir.clone());
        //tracing::info!("path:{:?}", path);
        if !self.dont_clone {
            //tracing::info!("start clone");
            clone(&path, git_url.as_ref()).await?;
            //tracing::info!("finish clone");
        }
        // finish cloning, store namespace ...
        //tracing::info!("start insert");
        insert_namespace_by_repo_path(path.to_str().unwrap().to_string(), namespace.clone());
        //tracing::info!("finish insert");
        tracing::trace!("Finish clone all the repos\n");
        Ok(path)
    }
}

async fn clone(path: &PathBuf, url: &str) -> Result<(), git2::Error> {
    if !path.is_dir() {
        //tracing::info!("Start cloning repo into {:?} from URL {}", path, url);
        Repository::clone(url, path)?;
        //tracing::info!("Finish cloning repo into {:?}", path);
    } else {
        tracing::info!("Directory {:?} is not empty, skipping Clone", path);
    }
    Ok(())
}

// /// Deprecated.
// /// If it migrate from a different system,
// /// the git record will change, and this is the reset function.
/*pub(crate) async fn hard_reset_to_head(repo_path: &PathBuf) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path).unwrap();
    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => {
            tracing::warn!("Repo {:?} does not have ref/heads/master", repo_path);
            return Ok(());
        }
    };
    let commit = repo.find_commit(
        head.target()
            .ok_or(git2::Error::from_str("HEAD does not point to a commit"))?,
    )?;

    // commit tree
    let tree = commit.tree()?;

    // Create CheckoutBuilder, set to force checkout to ensure changes to the working directory
    let mut checkout_opts = CheckoutBuilder::new();
    checkout_opts.force();

    // Correctly convert tree to Object before checking out the
    let tree_obj = tree.into_object();
    repo.checkout_tree(&tree_obj as &git2::Object, Some(&mut checkout_opts))?;
    tracing::info!(
        "Successfully checkout Repo {:?} into ref/heads/master",
        repo_path
    );

    Ok(())
}*/

/// return value: (tag_name, tree_id, commit_time)
pub(crate) async fn get_all_git_tags_with_time_sorted(
    repo_path: &PathBuf,
) -> Vec<(String, Oid, i64)> {
    let mut tags_with_dates = Vec::new();

    let repo = Repository::open(repo_path).unwrap();

    // Read the repository to get all tag names
    let tags: Vec<String> = repo
        .tag_names(None)
        .expect("Could not retrieve tags")
        .iter()
        .flatten()
        .map(|tag_name| tag_name.to_string())
        .collect();

    for tag_name in tags {
        let obj = repo
            .revparse_single(&("refs/tags/".to_owned() + &tag_name))
            .expect("Couldn't find tag object");

        // Convert annotated and light-weight tag into commit
        let commit = if let Some(tag) = obj.as_tag() {
            /*tag.target()
            .expect("Couldn't get tag target")
            .peel_to_commit()
            .expect("Couldn't peel to commit")*/
            let tmp = tag.target().expect("Couldn't get tag target");
            match tmp.peel_to_commit() {
                Ok(commit) => commit,
                Err(e) => {
                    tracing::info!("Couldn't peel to commit: {:?}", e);
                    continue;
                }
            }
        } else if let Some(commit) = obj.as_commit() {
            commit.clone()
        } else {
            //panic!("Error!");
            tracing::info!("Error");
            continue;
        };

        let commit_time = commit.time().seconds();

        let tree_id = match commit.tree() {
            Ok(tree) => tree.id(),
            _ => continue,
        };

        tags_with_dates.push((tag_name, tree_id, commit_time));
    }
    tags_with_dates.sort_by(|a, b| a.2.cmp(&b.2));
    tags_with_dates
}

pub(crate) async fn _print_all_tags(repo: &Repository, v: bool) {
    let tags = repo.tag_names(None).unwrap();

    let mut s = "".to_string();
    for tag_name in tags.iter().flatten() {
        let tag_ref = repo
            .find_reference(&format!("refs/tags/{tag_name}"))
            .unwrap();

        if v {
            if let Ok(tag_object) = tag_ref.peel_to_tag() {
                // Annotated tag
                let target_commit = tag_object.target().unwrap().peel_to_commit().unwrap();
                tracing::debug!(
                    "Annotated Tag: {}, Commit: {}, Message: {}",
                    tag_name,
                    target_commit.id(),
                    tag_object.message().unwrap_or("No message")
                );
            } else {
                let commit_object = tag_ref.peel(ObjectType::Commit).unwrap();
                let commit = commit_object
                    .into_commit()
                    .expect("Failed to peel into commit");
                tracing::debug!("Lightweight Tag: {}, Commit: {}", tag_name, commit.id());
            }
        } else {
            s += &format!("{tag_name}, ");
        }
    }

    tracing::debug!("TAGS {:?} tags: {}", repo.path(), s);
}

use clap::Parser;

use crate::utils::util;

#[derive(Parser, Debug)]
#[command(about = "Add file contents to the index")]
pub struct AddArgs {
    /// <pathspec>... Files to add content from.
    #[clap(required = false)]
    files: Vec<String>,

    /// Update the index not only where the working tree has a file matching <pathspec> but also where the index already has an entry. This adds, modifies, and removes index entries to match the working tree.
    ///
    /// If no <pathspec> is given when -A option is used, all files in the entire working tree are updated
    #[clap(short = 'A', long, group = "mode")]
    all: bool,

    /// Update the index just where it already has an entry matching <pathspec>.
    /// This removes as well as modifies index entries to match the working tree, but adds no new files.
    #[clap(short, long, group = "mode")]
    update: bool,
}

pub async fn execute(mut args: AddArgs) {
    if args.files.is_empty() {
        if !args.all && !args.update {
            println!("Nothing specified, nothing added.");
        } else {
            // add all files in the entire working tree
            args.files
                .push(util::working_dir().to_str().unwrap().to_owned().to_string());
        }
    }

    unimplemented!(); // TODO
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic]
    fn test_args_parse_update_conflict_with_all() {
        let _ = AddArgs::parse_from(["test", "-A", "-u"]);
    }
}

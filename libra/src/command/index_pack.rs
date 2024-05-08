use clap::Parser;

#[derive(Parser, Debug)]
pub struct IndexPackArgs {
    /// Pack file path
    pack_file: String,
    /// output index file path.
    /// Without this option the name of pack index file is constructed from
    /// the name of packed archive file by replacing `.pack` with `.idx`
    #[clap(short = 'o', required = false)]
    index_file: Option<String>, // Option is must, or clap will require it
}

pub fn execute(args: IndexPackArgs) {
    let pack_file = args.pack_file;
    let index_file = args.index_file.unwrap_or_else(|| {
        if !pack_file.ends_with(".pack") {
            eprintln!("fatal: pack-file does not end with '.pack'");
            return String::new();
        }
        pack_file.replace(".pack", ".idx")
    });
    if index_file.is_empty() {
        return;
    }
    if index_file == pack_file {
        eprintln!("fatal: pack-file and index-file are the same file");
        return;
    }

    build_index(&pack_file, &index_file);
}

fn build_index(pack_file: &str, index_file: &str) {
    // TODO
}
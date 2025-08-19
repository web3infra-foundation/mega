use clap::Parser;

#[derive(Parser, Debug)]
pub struct ReflogArgs {

}

enum Subcommands {

}

#[derive(Debug, Clone)]
enum RefName {
    Head(u64),
    Name(String),
}


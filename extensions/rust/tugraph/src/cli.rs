use structopt::StructOpt;

#[derive(StructOpt, Debug, Default, Clone)]
pub struct CratesProCli {
    #[structopt(subcommand)]
    pub(crate) _command: Option<Command>,

    #[structopt(short, long)]
    pub(crate) _mega_base: Option<String>,

    #[structopt(short, long)]
    pub(crate) dont_clone: bool,
}

#[derive(StructOpt, Debug, Default, Clone)]
pub enum Command {
    #[default]
    Mega,
}

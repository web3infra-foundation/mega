use clap::ValueEnum;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum DataSource {
    Mysql,
    Postgres,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum ZtmType {
    Agent,
    Relay,
}

impl std::str::FromStr for ZtmType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "agent" => Ok(ZtmType::Agent),
            "relay" => Ok(ZtmType::Relay),
            _ => Err(format!("'{}' is not a valid ztm type", s)),
        }
    }
}

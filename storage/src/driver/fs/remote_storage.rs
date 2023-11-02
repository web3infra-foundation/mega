

pub struct RemoteStorage {
    _host: String,
}

impl RemoteStorage {
    pub fn init(_host: String) -> RemoteStorage {
        RemoteStorage { _host }
    }
}
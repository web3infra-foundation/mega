//!
//!
//!
//!
//!

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use chrono::{DateTime, Duration, Utc};
use git::lfs::lfs_structs::Link;
use russh::server::{self, Auth, Msg, Session};
use russh::{Channel, ChannelId};
use russh_keys::key;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;

use storage::driver::database::storage::ObjectStorage;

use git::protocol::pack::{self};
use git::protocol::ServiceType;
use git::protocol::{PackProtocol, Protocol};

type ClientMap = HashMap<(usize, ChannelId), Channel<Msg>>;

#[derive(Clone)]
pub struct SshServer {
    pub client_pubkey: Arc<russh_keys::key::PublicKey>,
    pub clients: Arc<Mutex<ClientMap>>,
    pub id: usize,
    pub storage: Arc<dyn ObjectStorage>,
    // TODO: consider is it a good choice to bind data here, find a better solution to bind data with ssh client
    pub pack_protocol: Option<PackProtocol>,
}

impl server::Server for SshServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait]
impl server::Handler for SshServer {
    type Error = anyhow::Error;

    async fn channel_open_session(
        self,
        channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        tracing::info!("SshServer::channel_open_session:{}", channel.id());
        {
            let mut clients = self.clients.lock().unwrap();
            clients.insert((self.id, channel.id()), channel);
        }
        Ok((self, true, session))
    }

    /// # Executes a request on the SSH server.
    ///
    /// This function processes the received data from the specified channel and performs the
    /// corresponding action based on the received command.
    ///
    /// Arguments:
    /// - `self`: The current instance of the SSH server.
    /// - `channel`: The channel ID on which the request was received.
    /// - `data`: The received data from the channel.
    /// - `session`: The current SSH session.
    ///
    async fn exec_request(
        mut self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let data = String::from_utf8_lossy(data).trim().to_owned();
        tracing::info!("exec_request, channel:{:?}, command: {}", channel, data);
        // command exmaple:
        // Push: git-receive-pack '/path/to/repo.git'
        // Pull: git-upload-pack '/path/to/repo.git'
        // LFS HTTP Authenticate: git-lfs-authenticate '/path/to/repo.git' download/upload
        let command: Vec<_> = data.split(' ').collect();
        let path = command[1];
        let end = path.len() - ".git'".len();
        let mut pack_protocol = PackProtocol::new(
            PathBuf::from(&path[1..end]),
            self.storage.clone(),
            Protocol::Ssh,
        );
        match command[0] {
            "git-upload-pack" | "git-receive-pack" => {
                pack_protocol.service_type = ServiceType::from_str(command[0]).unwrap();
                let res = pack_protocol.git_info_refs().await;
                self.pack_protocol = Some(pack_protocol);
                session.data(channel, res.to_vec().into());
            }
            //Note that currently mega does not support pure ssh to transfer files, still relay on the https server.
            //see https://github.com/git-lfs/git-lfs/blob/main/docs/proposals/ssh_adapter.md for more details about pure ssh file transfer.
            "git-lfs-transfer" => {
                session.data(channel, "not implemented yet".as_bytes().to_vec().into());
            }
            // When connecting over SSH, the first attempt will be made to use
            // `git-lfs-transfer`, the pure SSH protocol, and if it fails, Git LFS will fall
            // back to the hybrid protocol using `git-lfs-authenticate`.
            "git-lfs-authenticate" => {
                let mut header = HashMap::new();
                header.insert("Accept".to_string(), "application/vnd.git-lfs".to_string());
                let link = Link {
                    href: "http://localhost:8000".to_string(),
                    header,
                    expires_at: {
                        let expire_time: DateTime<Utc> = Utc::now() + Duration::seconds(86400);
                        expire_time.to_rfc3339()
                    },
                };
                session.data(channel, serde_json::to_vec(&link).unwrap().into());
            }
            _ => println!("Not Supported command!"),
        }
        Ok((self, session))
    }

    async fn auth_publickey(
        self,
        user: &str,
        public_key: &key::PublicKey,
    ) -> Result<(Self, Auth), Self::Error> {
        tracing::info!("auth_publickey: {} / {:?}", user, public_key);
        Ok((self, Auth::Accept))
    }

    async fn auth_password(self, user: &str, password: &str) -> Result<(Self, Auth), Self::Error> {
        tracing::info!("auth_password: {} / {}", user, password);
        // in this example implementation, any username/password combination is accepted
        Ok((self, Auth::Accept))
    }

    async fn data(
        mut self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let pack_protocol = self.pack_protocol.as_mut().unwrap();

        match pack_protocol.service_type {
            ServiceType::UploadPack => {
                self.handle_upload_pack(channel, data, &mut session).await;
            }
            ServiceType::ReceivePack => {
                self.handle_receive_pack(channel, data, &mut session).await;
            }
        };

        Ok((self, session))
    }

    async fn channel_eof(
        self,
        channel: ChannelId,
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        {
            let mut clients = self.clients.lock().unwrap();
            clients.remove(&(self.id, channel));
        }
        session.exit_status_request(channel, 0000);
        session.close(channel);

        Ok((self, session))
    }
}

impl SshServer {
    async fn handle_upload_pack(&mut self, channel: ChannelId, data: &[u8], session: &mut Session) {
        let pack_protocol = self.pack_protocol.as_mut().unwrap();

        let (send_pack_data, buf) = pack_protocol
            .git_upload_pack(&mut Bytes::copy_from_slice(data))
            .await
            .unwrap();

        tracing::info!("buf is {:?}", buf);
        session.data(channel, String::from_utf8(buf.to_vec()).unwrap().into());

        let mut reader = send_pack_data.as_slice();
        loop {
            let mut temp = BytesMut::new();
            temp.reserve(65500);
            let length = reader.read_buf(&mut temp).await.unwrap();
            if temp.is_empty() {
                session.data(channel, pack::PKT_LINE_END_MARKER.to_vec().into());
                return;
            }
            let bytes_out = pack_protocol.build_side_band_format(temp, length);
            session.data(channel, bytes_out.to_vec().into());
        }
    }

    async fn handle_receive_pack(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) {
        let pack_protocol = self.pack_protocol.as_mut().unwrap();

        let buf = pack_protocol
            .git_receive_pack(Bytes::from(data.to_vec()))
            .await
            .unwrap();
        tracing::info!("report status: {:?}", buf);
        session.data(channel, buf.to_vec().into());
    }
}

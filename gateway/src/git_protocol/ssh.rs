use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use chrono::{DateTime, Duration, Utc};
use futures::StreamExt;
use russh::server::{self, Auth, Msg, Response, Session};
use russh::{Channel, ChannelId};
use russh_keys::key;
use tokio::io::AsyncReadExt;

use ceres::lfs::lfs_structs::Link;
use ceres::protocol::smart::{self};
use ceres::protocol::ServiceType;
use ceres::protocol::{SmartProtocol, TransportProtocol};
use jupiter::context::Context;

type ClientMap = HashMap<(usize, ChannelId), Channel<Msg>>;

#[derive(Clone)]
pub struct SshServer {
    pub client_pubkey: Arc<russh_keys::key::PublicKey>,
    pub clients: Arc<Mutex<ClientMap>>,
    pub id: usize,
    pub context: Context,
    // TODO: consider is it a good choice to bind data here, find a better solution to bind data with ssh client
    pub smart_protocol: Option<SmartProtocol>,
    pub data_combined: Vec<u8>,
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
        &mut self,
        channel: Channel<Msg>,
        _: &mut Session,
    ) -> Result<bool, Self::Error> {
        tracing::info!("SshServer::channel_open_session:{}", channel.id());
        {
            let mut clients = self.clients.lock().unwrap();
            clients.insert((self.id, channel.id()), channel);
        }
        Ok(true)
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
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let data = String::from_utf8_lossy(data).trim().to_owned();
        tracing::info!("exec_request, channel:{:?}, command: {}", channel, data);
        // command exmaple:
        // Push: git-receive-pack '/path/to/repo.git'
        // Pull: git-upload-pack '/path/to/repo.git'
        // LFS HTTP Authenticate: git-lfs-authenticate '/path/to/repo.git' download/upload
        let command: Vec<_> = data.split(' ').collect();
        let path = command[1];
        let path = path.replace(".git", "").replace('\'', "");
        let mut smart_protocol = SmartProtocol::new(
            PathBuf::from(&path),
            self.context.clone(),
            TransportProtocol::Ssh,
        );
        match command[0] {
            "git-upload-pack" | "git-receive-pack" => {
                smart_protocol.service_type = ServiceType::from_str(command[0]).unwrap();
                let res = smart_protocol.git_info_refs().await;
                self.smart_protocol = Some(smart_protocol);
                session.data(channel, res.to_vec().into());
                session.channel_success(channel);
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
                        let expire_time: DateTime<Utc> =
                            Utc::now() + Duration::try_seconds(86400).unwrap();
                        expire_time.to_rfc3339()
                    },
                };
                session.data(channel, serde_json::to_vec(&link).unwrap().into());
            }
            command => tracing::error!("Not Supported command! {}", command),
        }
        Ok(())
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &key::PublicKey,
    ) -> Result<Auth, Self::Error> {
        tracing::info!("auth_publickey: {} / {:?}", user, public_key);
        Ok(Auth::Accept)
    }

    async fn auth_keyboard_interactive(
        &mut self,
        _: &str,
        _: &str,
        _: Option<Response<'async_trait>>,
    ) -> Result<Auth, Self::Error> {
        tracing::info!("auth_keyboard_interactive");
        Ok(Auth::Accept)
    }

    // TODO! disable password auth
    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        tracing::info!("auth_password: {} / {}", user, password);
        // in this example implementation, any username/password combination is accepted
        Ok(Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let smart_protocol = self.smart_protocol.as_mut().unwrap();
        tracing::info!(
            "receiving data length:{}",
            // String::from_utf8_lossy(data),
            data.len()
        );

        match smart_protocol.service_type {
            ServiceType::UploadPack => {
                self.handle_upload_pack(channel, data, session).await;
            }
            ServiceType::ReceivePack => {
                self.data_combined.extend(data);
            }
        };
        session.channel_success(channel);
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(smart_protocol) = self.smart_protocol.as_mut() {
            if smart_protocol.service_type == ServiceType::ReceivePack {
                self.handle_receive_pack(channel, session).await;
            };
        }

        {
            let mut clients = self.clients.lock().unwrap();
            clients.remove(&(self.id, channel));
        }
        session.exit_status_request(channel, 0000);
        session.close(channel);
        Ok(())
    }
}

impl SshServer {
    async fn handle_upload_pack(&mut self, channel: ChannelId, data: &[u8], session: &mut Session) {
        let smart_protocol = self.smart_protocol.as_mut().unwrap();

        let (mut send_pack_data, buf) = smart_protocol
            .git_upload_pack(&mut Bytes::copy_from_slice(data))
            .await
            .unwrap();

        tracing::info!("buf is {:?}", buf);
        session.data(channel, String::from_utf8(buf.to_vec()).unwrap().into());

        while let Some(chunk) = send_pack_data.next().await {
            let mut reader = chunk.as_slice();
            loop {
                let mut temp = BytesMut::new();
                temp.reserve(65500);
                let length = reader.read_buf(&mut temp).await.unwrap();
                if length == 0 {
                    break;
                }
                let bytes_out = smart_protocol.build_side_band_format(temp, length);
                session.data(channel, bytes_out.to_vec().into());
            }
        }
        session.data(channel, smart::PKT_LINE_END_MARKER.to_vec().into());
    }

    async fn handle_receive_pack(&mut self, channel: ChannelId, session: &mut Session) {
        let smart_protocol = self.smart_protocol.as_mut().unwrap();

        let buf = smart_protocol
            .git_receive_pack(Bytes::from(self.data_combined.to_vec()))
            .await
            .unwrap();
        tracing::info!("report status: {:?}", buf);
        session.data(channel, buf.to_vec().into());
    }
}

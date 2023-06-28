//!
//!
//!
//!
//!

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use russh::server::{self, Auth, Msg, Session};
use russh::{Channel, ChannelId};

use database::driver::ObjectStorage;
use russh_keys::key;
use std::collections::HashMap;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, BufReader};

use crate::protocol::ServiceType;

use super::pack::{self};
use super::{PackProtocol, Protocol};

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
        tracing::info!("exec: {:?},{}", channel, data);
        let res = self.handle_git_command(&data).await;
        session.data(channel, res.into());
        Ok((self, session))
    }

    async fn auth_publickey(
        self,
        user: &str,
        public_key: &key::PublicKey,
    ) -> Result<(Self, Auth), Self::Error> {
        tracing::info!("auth_publickey: {} / {:?}", user, public_key);
        Ok((self, server::Auth::Accept))
    }

    async fn auth_password(self, user: &str, password: &str) -> Result<(Self, Auth), Self::Error> {
        tracing::info!("auth_password: {} / {}", user, password);
        // in this example implementation, any username/password combination is accepted
        Ok((self, server::Auth::Accept))
    }

    async fn data(
        mut self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let pack_protocol = self.pack_protocol.as_mut().unwrap();
        let data_str = String::from_utf8_lossy(data).trim().to_owned();
        tracing::info!(
            "SSH: client sends data: {:?}, channel:{}",
            data_str,
            channel
        );
        match pack_protocol.service_type {
            Some(ServiceType::UploadPack) => {
                self.handle_upload_pack(channel, data, &mut session).await;
            }
            Some(ServiceType::ReceivePack) => {
                self.handle_receive_pack(channel, data, &mut session).await;
            }
            None => panic!(),
        };
        Ok((self, session))
    }

    async fn channel_eof(
        self,
        channel: ChannelId,
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        // session.close(channel);
        // match session.flush() {
        //     Ok(_) => {},
        //     Err(e) => println!("Error flushing session: {:?}", e),
        // }
        // session.disconnect(Disconnect::ByApplication, "channel close", "en");
        // match session.disconnect(None, "Closing session") {
        //     Ok(_) => {},
        //     Err(e) => println!("Error disconnecting session: {:?}", e),
        // }
        session.close(channel);
        Ok((self, session))
    }

    // async fn channel_close(
    //     self,
    //     channel: ChannelId,
    //     session: Session,
    // ) -> Result<(Self, Session), Self::Error> {
    //     tracing::info!("channel_close: {:?}", channel);
    //     Ok((self, session))
    // }
}

impl SshServer {
    async fn handle_git_command(&mut self, command: &str) -> String {
        let command: Vec<_> = command.split(' ').collect();
        // command:
        // Push: git-receive-pack '/root/repotest/src.git'
        // Pull: git-upload-pack '/root/repotest/src.git'
        let path = command[1];
        let end = path.len() - ".git'".len();
        let mut pack_protocol = PackProtocol::new(
            PathBuf::from(&path[1..end]),
            self.storage.clone(),
            Protocol::Ssh,
        );
        let res = pack_protocol
            .git_info_refs(ServiceType::from_str(command[0]).unwrap())
            .await;

        self.pack_protocol = Some(pack_protocol);
        String::from_utf8(res.to_vec()).unwrap()
    }

    async fn handle_upload_pack(&mut self, channel: ChannelId, data: &[u8], session: &mut Session) {
        let pack_protocol = self.pack_protocol.as_mut().unwrap();

        let (send_pack_data, buf) = pack_protocol
            .git_upload_pack(&mut Bytes::copy_from_slice(data))
            .await
            .unwrap();

        tracing::info!("buf is {:?}", buf);
        session.data(channel, String::from_utf8(buf.to_vec()).unwrap().into());

        let mut reader = BufReader::new(send_pack_data.as_slice());
        loop {
            let mut temp = BytesMut::new();
            let length = reader.read_buf(&mut temp).await.unwrap();
            if temp.is_empty() {
                let mut bytes_out = BytesMut::new();
                bytes_out.put_slice(pack::PKT_LINE_END_MARKER);
                tracing::info!("send: ends: {:?}", bytes_out.clone().freeze());
                session.data(channel, bytes_out.to_vec().into());
                return;
            }
            let bytes_out = pack_protocol.build_side_band_format(temp, length);
            tracing::info!("send: bytes_out: {:?}", bytes_out.clone().freeze());
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
        if !buf.is_empty() {
            tracing::info!("report status: {:?}", buf);
            session.data(channel, buf.to_vec().into());
        }
    }
}

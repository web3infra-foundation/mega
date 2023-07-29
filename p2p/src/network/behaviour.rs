use async_trait::async_trait;
use futures::{io, AsyncRead, AsyncWrite, AsyncWriteExt};
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName};
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{dcutr, identify, relay, rendezvous, request_response};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event")]
pub struct Behaviour {
    pub relay_client: relay::client::Behaviour,
    pub identify: identify::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub rendezvous: rendezvous::client::Behaviour,
    pub request_response: request_response::Behaviour<FileExchangeCodec>,
}

#[derive(Debug, Clone)]
pub struct FileExchangeProtocol();
#[derive(Clone)]
pub struct FileExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRequest(pub String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResponse(pub Vec<u8>);

impl ProtocolName for FileExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/file-exchange/mega".as_bytes()
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Event {
    Identify(identify::Event),
    RelayClient(relay::client::Event),
    Dcutr(dcutr::Event),
    Kademlia(KademliaEvent),
    Rendezvous(rendezvous::client::Event),
    RequestResponse(request_response::Event<FileRequest, FileResponse>),
}

#[async_trait]
impl request_response::Codec for FileExchangeCodec {
    type Protocol = FileExchangeProtocol;
    type Request = FileRequest;
    type Response = FileResponse;

    async fn read_request<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let vec = read_length_prefixed(io, 1_000_000).await?;

        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        Ok(FileRequest(String::from_utf8(vec).unwrap()))
    }

    async fn read_response<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let vec = read_length_prefixed(io, 500_000_000).await?; // update transfer maximum

        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        Ok(FileResponse(vec))
    }

    async fn write_request<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
        FileRequest(data): FileRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_length_prefixed(io, data).await?;
        io.close().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
        FileResponse(data): FileResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_length_prefixed(io, data).await?;
        io.close().await?;

        Ok(())
    }
}

impl From<identify::Event> for Event {
    fn from(e: identify::Event) -> Self {
        Event::Identify(e)
    }
}

impl From<relay::client::Event> for Event {
    fn from(e: relay::client::Event) -> Self {
        Event::RelayClient(e)
    }
}

impl From<dcutr::Event> for Event {
    fn from(e: dcutr::Event) -> Self {
        Event::Dcutr(e)
    }
}

impl From<KademliaEvent> for Event {
    fn from(e: KademliaEvent) -> Self {
        Event::Kademlia(e)
    }
}

impl From<rendezvous::client::Event> for Event {
    fn from(e: rendezvous::client::Event) -> Self {
        Event::Rendezvous(e)
    }
}

impl From<request_response::Event<FileRequest, FileResponse>> for Event {
    fn from(event: request_response::Event<FileRequest, FileResponse>) -> Self {
        Event::RequestResponse(event)
    }
}

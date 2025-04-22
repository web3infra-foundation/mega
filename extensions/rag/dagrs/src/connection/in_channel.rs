use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::node::node::NodeId;

use super::information_packet::Content;

/// # Input Channels
/// A hash-table mapping `NodeId` to `InChannel`. In **Dagrs**, each `Node` stores input
/// channels in this map, enabling `Node` to receive information packets from other `Node`s.
#[derive(Default)]
pub struct InChannels(pub(crate) HashMap<NodeId, Arc<Mutex<InChannel>>>);

impl InChannels {
    /// Perform a blocking receive on the incoming channel from `NodeId`.
    pub fn blocking_recv_from(&mut self, id: &NodeId) -> Result<Content, RecvErr> {
        match self.get(id) {
            Some(channel) => channel.blocking_lock().blocking_recv(),
            None => Err(RecvErr::NoSuchChannel),
        }
    }
    /// Perform a asynchronous receive on the incoming channel from `NodeId`.
    pub async fn recv_from(&mut self, id: &NodeId) -> Result<Content, RecvErr> {
        match self.get(id) {
            Some(channel) => channel.lock().await.recv().await,
            None => Err(RecvErr::NoSuchChannel),
        }
    }

    /// Calls `blocking_recv` for all the [`InChannel`]s, and applies transformation `f` to
    /// the return values of the call.
    pub fn blocking_map<F, T>(&mut self, mut f: F) -> Vec<T>
    where
        F: FnMut(Result<Content, RecvErr>) -> T,
    {
        self.keys()
            .into_iter()
            .map(|id| f(self.blocking_recv_from(&id)))
            .collect()
    }

    /// Calls `recv` for all the [`InChannel`]s, and applies transformation `f` to
    /// the return values of the call asynchronously.
    pub async fn map<F, T>(&mut self, mut f: F) -> Vec<T>
    where
        F: FnMut(Result<Content, RecvErr>) -> T,
    {
        let futures = self
            .0
            .iter_mut()
            .map(|(_, c)| async { c.lock().await.recv().await });
        join_all(futures).await.into_iter().map(|x| f(x)).collect()
    }

    /// Close the channel by the given `NodeId` asynchronously, and remove the channel in this map.
    pub async fn close_async(&mut self, id: &NodeId) {
        if let Some(c) = self.get(id) {
            c.lock().await.close();
            self.0.remove(id);
        }
    }

    /// Close the channel by the given `NodeId`, and remove the channel in this map.
    pub fn close(&mut self, id: &NodeId) {
        if let Some(c) = self.get(id) {
            c.blocking_lock().close();
            self.0.remove(id);
        }
    }

    pub(crate) fn insert(&mut self, node_id: NodeId, channel: Arc<Mutex<InChannel>>) {
        self.0.insert(node_id, channel);
    }

    pub(crate) fn close_all(&mut self) {
        self.0.values_mut().for_each(|c| c.blocking_lock().close());
    }

    fn get(&self, id: &NodeId) -> Option<Arc<Mutex<InChannel>>> {
        match self.0.get(id) {
            Some(c) => Some(c.clone()),
            None => None,
        }
    }

    fn keys(&self) -> Vec<NodeId> {
        self.0.keys().map(|x| *x).collect()
    }
}

/// # Input Channel
/// Wrapper of receivers of `tokio::sync::mpsc` and `tokio::sync::broadcast`. **Dagrs** will
/// decide the inner type of channel when building the graph.
/// Learn more about [Tokio Channels](https://tokio.rs/tokio/tutorial/channels).
pub enum InChannel {
    /// Receiver of a `tokio::sync::mpsc` channel.
    Mpsc(mpsc::Receiver<Content>),
    /// Receiver of a `tokio::sync::broadcast` channel.
    Bcst(broadcast::Receiver<Content>),
}

impl InChannel {
    /// Perform a blocking receive on this channel.
    fn blocking_recv(&mut self) -> Result<Content, RecvErr> {
        match self {
            InChannel::Mpsc(receiver) => {
                if let Some(content) = receiver.blocking_recv() {
                    Ok(content)
                } else {
                    Err(RecvErr::Closed)
                }
            }
            InChannel::Bcst(receiver) => match receiver.blocking_recv() {
                Ok(v) => Ok(v),
                Err(e) => match e {
                    broadcast::error::RecvError::Closed => Err(RecvErr::Closed),
                    broadcast::error::RecvError::Lagged(x) => Err(RecvErr::Lagged(x)),
                },
            },
        }
    }

    /// Perform a asynchronous receive on this channel.
    async fn recv(&mut self) -> Result<Content, RecvErr> {
        match self {
            InChannel::Mpsc(receiver) => {
                if let Some(content) = receiver.recv().await {
                    Ok(content)
                } else {
                    Err(RecvErr::Closed)
                }
            }
            InChannel::Bcst(receiver) => match receiver.recv().await {
                Ok(v) => Ok(v),
                Err(e) => match e {
                    broadcast::error::RecvError::Closed => Err(RecvErr::Closed),
                    broadcast::error::RecvError::Lagged(x) => Err(RecvErr::Lagged(x)),
                },
            },
        }
    }

    /// Close the channel and drop the messages inside.
    fn close(&mut self) {
        match self {
            InChannel::Mpsc(receiver) => receiver.close(),
            // Broadcast channel will be closed after `self` is dropped.
            InChannel::Bcst(_) => (),
        }
    }
}

/// # Input Channel Error Types
/// - NoSuchChannel: try to get a channel with an invalid `NodeId`.
/// - Closed: the channel to receive messages from is closed and empty already.
/// - Lagged(x): the channel encounters a cache overflow and `x` information
/// pakages are dropped on this receiver's side.
#[derive(Debug)]
pub enum RecvErr {
    NoSuchChannel,
    Closed,
    Lagged(u64),
}

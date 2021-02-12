// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::{application::*, node::*, stage::ReportersHandles};

use std::{
    collections::HashMap,
    net::SocketAddr,
    ops::{Deref, DerefMut},
};

mod event_loop;
mod init;
mod terminating;

// Cluster builder
builder!(ClusterBuilder {
    reporter_count: u8,
    thread_count: usize,
    data_centers: Vec<String>,
    buffer_size: usize,
    recv_buffer_size: Option<u32>,
    send_buffer_size: Option<u32>,
    authenticator: PasswordAuth
});
/// ClusterHandle to be passed to the children (Node)
#[derive(Clone)]
pub struct ClusterHandle {
    tx: mpsc::UnboundedSender<ClusterEvent>,
}
/// ClusterInbox is used to recv events
pub struct ClusterInbox {
    rx: mpsc::UnboundedReceiver<ClusterEvent>,
}

impl Deref for ClusterHandle {
    type Target = mpsc::UnboundedSender<ClusterEvent>;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

impl DerefMut for ClusterHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tx
    }
}

// Cluster state
pub struct Cluster {
    service: Service,
    reporter_count: u8,
    thread_count: usize,
    data_centers: Vec<String>,
    buffer_size: usize,
    recv_buffer_size: Option<u32>,
    send_buffer_size: Option<u32>,
    authenticator: PasswordAuth,
    nodes: HashMap<SocketAddr, NodeInfo>,
    // registry: Registry,
    // arc_ring: Option<ArcRing>,
    // weak_rings: Vec<Box<WeakRing>>,
    handle: Option<ClusterHandle>,
    inbox: ClusterInbox,
}

impl Cluster {
    pub(crate) fn clone_handle(&self) -> ClusterHandle {
        self.handle.clone().unwrap()
    }
}
// Cluster Event type
pub enum ClusterEvent {
    RegisterReporters(Service, HashMap<SocketAddr, ReportersHandles>),
    Service(Service),
    AddNode(SocketAddr),
    RemoveNode(SocketAddr),
    TryBuild(u8),
    Shutdown,
}

impl<H: ScyllaScope> ActorBuilder<ScyllaHandle<H>> for ClusterBuilder {}

/// implementation of builder
impl Builder for ClusterBuilder {
    type State = Cluster;
    fn build(self) -> Self::State {
        let (tx, rx) = mpsc::unbounded_channel::<ClusterEvent>();
        let handle = Some(ClusterHandle { tx });
        let inbox = ClusterInbox { rx };
        // TODO initialize global_ring

        Self::State {
            service: Service::new(),
            reporter_count: self.reporter_count.unwrap(),
            thread_count: self.thread_count.unwrap(),
            data_centers: self.data_centers.unwrap(),
            buffer_size: self.buffer_size.unwrap(),
            recv_buffer_size: self.recv_buffer_size.unwrap(),
            send_buffer_size: self.send_buffer_size.unwrap(),
            authenticator: self.authenticator.unwrap(),
            nodes: HashMap::new(),
            handle,
            inbox,
        }
        .set_name()
    }
}

/// `NodeInfo` contains the field to identify a ScyllaDB node.
pub struct NodeInfo {
    address: SocketAddr,
    /// in which data_center the scylla node exist
    data_center: String,
    /// it's the node handle for the Node supervisor tree
    node_handle: NodeHandle,
    /// The tokens of all nodes shards.
    tokens: Vec<i64>,
    /// the shard_count in scylla node.
    shard_count: u16,
    /// the most significant bit
    msb: u8,
}

/// impl name of the Cluster
impl Name for Cluster {
    fn set_name(mut self) -> Self {
        self.service.update_name("Cluster".to_string());
        self
    }
    fn get_name(&self) -> String {
        self.service.get_name()
    }
}

#[async_trait::async_trait]
impl<H: ScyllaScope> AknShutdown<Cluster> for ScyllaHandle<H> {
    async fn aknowledge_shutdown(self, mut _state: Cluster, _status: Result<(), Need>) {
        _state.service.update_status(ServiceStatus::Stopped);
        let event = ScyllaEvent::Children(ScyllaChild::Cluster(_state.service.clone()));
        let _ = self.send(event);
    }
}

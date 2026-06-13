/// Sync topology. SCOPE.md makes peer-vs-broker a configuration choice,
/// not two code paths: the same event model rides either. `Peer` joins
/// the flat multicast/gossip mesh for a small team; `Client` dials a
/// router/broker hub for a larger org.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Topology {
    /// Direct peer-to-peer; teammates discover each other on the mesh.
    Peer,
    /// Dial one or more router/broker endpoints (e.g.
    /// `tcp/hub.example:7447`).
    Client { endpoints: Vec<String> },
}

/// Minimal sync configuration: which workspace this node replicates and
/// the topology to join. The workspace scopes every Zenoh key
/// (`lazyboy/{workspace}/...`) so distinct workspaces never cross.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncConfig {
    pub workspace: String,
    pub topology: Topology,
}

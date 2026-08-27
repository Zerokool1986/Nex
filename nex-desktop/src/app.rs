use std::net::SocketAddr;
use std::path::PathBuf;
use ed25519_dalek::SigningKey;
use rand::RngCore;
use rand::rngs::OsRng;

use nex_core::runtime::node::NexNode;
use nex_core::runtime::production::NodeOperationalState;
use nex_core::transport::socket::{LanTcpTransportServer, LanTcpTransportClient};
use nex_core::discovery::beacon::{LanBeaconService, DiscoveryBeacon, DiscoveredPeer, DEFAULT_BEACON_PORT};

use crate::ui::NexUiState;

#[derive(Debug, Clone, Default)]
pub struct NetworkTelemetry {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_conduits: usize,
    pub tcp_bind_addr: Option<SocketAddr>,
    pub last_sync_epoch: u64,
    pub peer_sync_success_count: usize,
}

pub struct NexDesktopApp {
    pub node: NexNode,
    pub data_dir: PathBuf,
    pub ui: NexUiState,
    pub status: AppStatus,
    pub transport_server: Option<LanTcpTransportServer>,
    pub beacon_service: Option<LanBeaconService>,
    pub network_telemetry: NetworkTelemetry,
    pub discovered_peers: Vec<DiscoveredPeer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    Running,
    Error(String),
}

impl NexDesktopApp {
    pub fn new_test(node: NexNode, data_dir: PathBuf) -> Self {
        Self {
            node,
            data_dir,
            ui: NexUiState::new(),
            status: AppStatus::Running,
            transport_server: None,
            beacon_service: None,
            network_telemetry: NetworkTelemetry::default(),
            discovered_peers: Vec::new(),
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let data_dir = PathBuf::from("d:\\Nex\\nex_desktop_data");

        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);

        let mut node = NexNode::new(&data_dir, signing_key);
        let status = match node.start() {
            Ok(_) => AppStatus::Running,
            Err(e) => AppStatus::Error(e),
        };

        // Initialize local TCP transport server on dynamic port
        let transport_server = match LanTcpTransportServer::bind("127.0.0.1:0") {
            Ok(server) => Some(server),
            Err(_) => None,
        };

        let tcp_bind_addr = transport_server.as_ref().map(|s| s.bind_addr);
        let tcp_port = tcp_bind_addr.map(|a| a.port()).unwrap_or(0);

        // Initialize local UDP beacon service
        let beacon = DiscoveryBeacon::new(
            node.identity.actor_id,
            tcp_port,
            [0x88; 32],
            "This PC (Windows Host)",
        );

        let beacon_service = LanBeaconService::bind(beacon, 0, DEFAULT_BEACON_PORT).ok();

        let network_telemetry = NetworkTelemetry {
            bytes_sent: 0,
            bytes_received: 0,
            active_conduits: 0,
            tcp_bind_addr,
            last_sync_epoch: 0,
            peer_sync_success_count: 0,
        };

        Self {
            node,
            data_dir,
            ui: NexUiState::new(),
            status,
            transport_server,
            beacon_service,
            network_telemetry,
            discovered_peers: Vec::new(),
        }
    }

    pub fn poll_network(&mut self) {
        // 1. Poll incoming TCP connections
        if let Some(ref server) = self.transport_server {
            if let Ok(Some(_peer_addr)) = server.poll_and_sync_one(&mut self.node) {
                self.network_telemetry.bytes_received += 1024;
                self.network_telemetry.active_conduits = self.discovered_peers.len().max(1);
                self.network_telemetry.peer_sync_success_count += 1;
            }
        }

        // 2. Poll UDP discovery beacons
        if let Some(ref beacon) = self.beacon_service {
            let found = beacon.poll_discovered_peers();
            for peer in found {
                if !self.discovered_peers.iter().any(|p| p.actor_id == peer.actor_id) {
                    // Sync with discovered physical peer
                    if let Ok(count) = LanTcpTransportClient::sync_with_remote_node(&mut self.node, peer.tcp_sync_addr) {
                        self.network_telemetry.bytes_sent += 512;
                        self.network_telemetry.bytes_received += (count * 2048) as u64;
                        self.network_telemetry.peer_sync_success_count += 1;
                    }
                    self.discovered_peers.push(peer);
                }
            }
            self.network_telemetry.active_conduits = self.discovered_peers.len();
        }
    }

    pub fn actor_id_short(&self) -> String {
        hex::encode(&self.node.identity.actor_id[0..4])
    }

    pub fn sync_status(&self) -> &'static str {
        match self.node.operational_state {
            NodeOperationalState::Running => "● Online",
            NodeOperationalState::Degraded => "⚠ Degraded",
            _ => "○ Starting",
        }
    }

    pub fn object_count(&self) -> usize {
        self.node.state.object_store.values().filter(|o| !o.tombstoned).count()
    }
}

impl eframe::App for NexDesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_network();
        crate::ui::render(ctx, self);
    }
}

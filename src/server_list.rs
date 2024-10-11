
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use valence::network::{async_trait, HandshakeData, PlayerSampleEntry, ServerListPing};
use valence::{prelude::*, MINECRAFT_VERSION};

pub struct ServerList;

impl Plugin for ServerList {
    fn build(&self, app: &mut App) {
        let list = Arc::<Mutex<Vec<PlayerSampleEntry>>>::default();
        app
            .insert_resource(PlayerList(list.clone()))
            .insert_resource(NetworkSettings {
                connection_mode: ConnectionMode::Online { prevent_proxy_connections: false },
                callbacks: MyCallbacks(list).into(),
                ..Default::default()
            })
            .add_systems(Update, update_player_list);
    }
}

struct MyCallbacks(Arc<Mutex<Vec<PlayerSampleEntry>>>);

#[derive(Resource)]
pub struct PlayerList(Arc<Mutex<Vec<PlayerSampleEntry>>>);

#[async_trait]
impl NetworkCallbacks for MyCallbacks {
    async fn server_list_ping(
        &self,
        shared: &SharedNetworkState,
        _remote_addr: SocketAddr,
        handshake_data: &HandshakeData,
    ) -> ServerListPing {
        ServerListPing::Respond {
            online_players: shared.player_count().load(Ordering::Relaxed) as i32,
            max_players: shared.max_players() as i32,
            player_sample: self.0.lock().unwrap().clone(),
            description: "BongoThirteen's Experimental Server".color(Color::DARK_AQUA),
            favicon_png: include_bytes!("../valence/assets/logo-64x64.png"),
            version_name: ("Valence ".color(Color::GOLD) + MINECRAFT_VERSION.color(Color::RED))
                .to_legacy_lossy(),
            protocol: handshake_data.protocol_version,
        }
    }
}

fn update_player_list(
    shared: Res<PlayerList>,
    entries: Query<(&UniqueId, &Username), With<PlayerListEntry>>,
) {
    let list = entries.iter().map(|(uuid, name)| {
        PlayerSampleEntry {
            name: name.0.clone(),
            id: uuid.0,
        }
    }).collect();
    *shared.0.lock().unwrap() = list;
}

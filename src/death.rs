
use valence::{entity::living::Health, message::SendMessage, prelude::*};

#[derive(Event, Debug, Copy, Clone)]
pub struct DeathEvent {
    pub entity: Entity,
    pub killed_by: Option<Entity>,
}

pub struct Death;

impl Plugin for Death {
    fn build(&self, app: &mut App) {
        app.add_event::<DeathEvent>()
            .add_systems(Update, handle_death_event);
    }
}

fn handle_death_event(
    mut events: EventReader<DeathEvent>,
    mut clients: Query<(&mut Client, &Username, &EntityLayerId)>,
    entities: Query<&EntityLayerId, (With<Health>, Without<Client>)>,
    mut layers: Query<&mut EntityLayer>,
    mut commands: Commands,
) {
    for event in events.read() {
        if let Some([(mut client, killed_name, killed_layer_id), (_, killer_name, killer_layer_id)]) = event.killed_by.and_then(|k| clients.get_many_mut([event.entity, k]).ok()) {
            if killer_layer_id != killed_layer_id {
                continue;
            }
            client.kill(format!("You were killed by {}", killer_name.0));
            if let Ok(mut layer) = layers.get_mut(killer_layer_id.0) {
                layer.send_chat_message(format!("{} was killed by {}", killed_name.0, killer_name.0).color(Color::DARK_AQUA));
            }
        } else if let Ok((mut client, killed_name, layer_id)) = clients.get_mut(event.entity) {
            client.kill("You were killed by an unknown entity");
            if let Ok(mut layer) = layers.get_mut(layer_id.0) {
                layer.send_chat_message(format!("{} was killed", killed_name.0).color(Color::DARK_AQUA));
            }
        } else if let Ok(_) = entities.get(event.entity) {
            commands.entity(event.entity).insert(Despawned);
        }
    }
}

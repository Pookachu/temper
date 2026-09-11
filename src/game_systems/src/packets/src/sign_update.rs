use bevy_ecs::system::Res;
use temper_protocol::SignUpdateReceiver;

pub fn handle(receiver: Res<SignUpdateReceiver>) {
    for (event, eid) in receiver.0.try_iter() {
        tracing::info!("SignUpdate from {:?}: {:?}", eid, event);
    }
}

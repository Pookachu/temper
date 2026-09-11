use bevy_ecs::{
    message::MessageReader,
    system::{Query, Res},
};

use temper_codec::net_types::{network_position::NetworkPosition, var_int::VarInt};
use temper_components::player::position::Position;
use temper_core::pos::BlockPos;
use temper_messages::BlockEntityPlaced;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::{
    SignUpdateReceiver,
    outgoing::{block_entity_data::BlockEntityDataPacket, open_sign_editor::OpenSignEditor},
};
use temper_state::GlobalStateResource;
use temper_text::{TextComponent, TextContent};
use temper_world::Dimension;
use temper_world_format::{BlockEntityKind, SignBlockEntity};

use tracing::{error, trace};

pub fn handle_sign_placed(
    mut events: MessageReader<BlockEntityPlaced>,
    query: Query<&StreamWriter>,
) {
    for event in events.read() {
        if event.kind != BlockEntityKind::Sign {
            continue;
        }

        let Ok(conn) = query.get(event.player) else {
            continue;
        };

        if let Err(err) = conn.send_packet(OpenSignEditor {
            location: NetworkPosition {
                x: event.position.pos.x,
                y: event.position.pos.y as i16,
                z: event.position.pos.z,
            },
            is_front_text: true,
        }) {
            error!("Failed to send open sign editor packet: {:?}", err);
        }
    }
}

/// Stores text the client sent back after closing the sign editor.
pub fn handle_sign_update(
    receiver: Res<SignUpdateReceiver>,
    state: Res<GlobalStateResource>,
    players: Query<(&StreamWriter, &Position)>,
) {
    for (event, eid) in receiver.0.try_iter() {
        let block_pos: BlockPos = event.position.into();

        let Ok(chunk) = state
            .0
            .world
            .get_chunk(block_pos.chunk(), Dimension::Overworld)
        // todo: dimensions
        else {
            error!("Failed to get chunk for sign update at {block_pos}");
            continue;
        };

        let Some(mut entry) = chunk.block_entities.get_mut(&block_pos.chunk_block_pos()) else {
            trace!("Sign update from {eid:?} for a position with no block entity: {block_pos}");
            continue;
        };

        if entry.kind != BlockEntityKind::Sign {
            trace!("Sign update from {eid:?} for a non-sign block entity: {block_pos}");
            continue;
        }

        let mut sign: SignBlockEntity = match serde_json::from_slice(&entry.blob) {
            Ok(sign) => sign,
            Err(err) => {
                error!("Failed to read sign at {block_pos}: {err}");
                continue;
            }
        };

        if sign.is_waxed {
            trace!("Sign update from {eid:?} for a waxed sign: {block_pos}");
            continue;
        }

        let text = if event.is_front_text {
            &mut sign.front_text
        } else {
            &mut sign.back_text
        };

        text.messages = [&event.line_1, &event.line_2, &event.line_3, &event.line_4]
            .into_iter()
            .map(|line| TextComponent {
                content: TextContent::Text { text: line.clone() },
                ..Default::default()
            })
            .collect();

        let blob = match sign.to_blob() {
            Ok(blob) => blob,
            Err(err) => {
                error!("Failed to write sign at {block_pos}: {err}");
                continue;
            }
        };
        let protocol_id = entry.protocol_id;
        entry.blob = blob.clone();

        drop(entry);
        chunk.mark_dirty();

        let nbt = match BlockEntityKind::Sign.to_network_nbt(&blob) {
            Ok(nbt) => nbt,
            Err(err) => {
                error!("Failed to build sign nbt for {block_pos}: {err}");
                continue;
            }
        };

        let packet = BlockEntityDataPacket {
            location: NetworkPosition {
                x: block_pos.pos.x,
                y: block_pos.pos.y as i16,
                z: block_pos.pos.z,
            },
            entity_type: VarInt::new(i32::from(protocol_id)),
            nbt,
        };

        let block_chunk = block_pos.chunk();
        let render_distance = state.0.config.chunk_render_distance as i32;

        for (conn, pos) in players.iter() {
            let player_chunk = pos.chunk();
            if (block_chunk.x() - player_chunk.x()).abs() <= render_distance
                && (block_chunk.z() - player_chunk.z()).abs() <= render_distance
                && let Err(err) = conn.send_packet_ref(&packet)
            {
                error!("Failed to send block entity data packet: {:?}", err);
            }
        }
    }
}

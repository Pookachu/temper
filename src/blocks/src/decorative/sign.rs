use crate::BlockBehavior;
use temper_block_data::{PlacedBlocks, PlacementContext};
use temper_blocks_generated::SignBlock;
use temper_core::block_state_id::BlockStateId;
use temper_macros::match_block;

impl BlockBehavior for SignBlock {
    fn get_placement_state(&mut self, context: PlacementContext) -> PlacedBlocks {
        let block = context
            .level
            .get_chunk(context.block_pos.chunk(), context.dimension)
            .map(|c| c.get_block(context.block_pos.chunk_block_pos()))
            .unwrap_or(BlockStateId::new(0));

        self.waterlogged = match_block!("water", block);
        self.rotation = yaw_to_sign_rotation(context.player_rotation.yaw);

        PlacedBlocks::default()
    }
}

/// Standing signs use a 16-step rotation rather than a 4-direction facing.
/// Rotation 0 is south, increasing counter-clockwise in 22.5 degree steps —
/// the same convention as `Direction::from_yaw`, but the value is where the
/// sign's face points, so it's the player's yaw turned around.
fn yaw_to_sign_rotation(yaw: f32) -> i32 {
    ((yaw / 22.5 + 0.5).floor() as i32 + 8).rem_euclid(16)
}

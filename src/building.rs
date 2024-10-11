use valence::entity::entity::Flags;
use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::{prelude::*, Direction};

use valence::entity::{
    blaze::BlazeEntityBundle,
    creeper::CreeperEntityBundle,
    elder_guardian::ElderGuardianEntityBundle,
    ender_dragon::EnderDragonEntityBundle,
    endermite::EndermiteEntityBundle,
    evoker::EvokerEntityBundle,
    ghast::GhastEntityBundle,
    guardian::GuardianEntityBundle,
    hoglin::HoglinEntityBundle,
    husk::HuskEntityBundle,
    magma_cube::MagmaCubeEntityBundle,
    phantom::PhantomEntityBundle,
    piglin_brute::PiglinBruteEntityBundle,
    pillager::PillagerEntityBundle,
    ravager::RavagerEntityBundle,
    shulker::ShulkerEntityBundle,
    silverfish::SilverfishEntityBundle,
    skeleton::SkeletonEntityBundle,
    slime::SlimeEntityBundle,
    stray::StrayEntityBundle,
    vex::VexEntityBundle,
    vindicator::VindicatorEntityBundle,
    warden::WardenEntityBundle,
    witch::WitchEntityBundle,
    wither::WitherEntityBundle,
    wither_skeleton::WitherSkeletonEntityBundle,
    zoglin::ZoglinEntityBundle,
    zombie::ZombieEntityBundle,
    zombie_villager::ZombieVillagerEntityBundle,
    bee::BeeEntityBundle,
    cave_spider::CaveSpiderEntityBundle,
    enderman::EndermanEntityBundle,
    dolphin::DolphinEntityBundle,
    drowned::DrownedEntityBundle,
    fox::FoxEntityBundle,
    goat::GoatEntityBundle,
    iron_golem::IronGolemEntityBundle,
    llama::LlamaEntityBundle,
    panda::PandaEntityBundle,
    piglin::PiglinEntityBundle,
    polar_bear::PolarBearEntityBundle,
    spider::SpiderEntityBundle,
    trader_llama::TraderLlamaEntityBundle,
    wolf::WolfEntityBundle,
    zombified_piglin::ZombifiedPiglinEntityBundle,
    allay::AllayEntityBundle,
    axolotl::AxolotlEntityBundle,
    bat::BatEntityBundle,
    camel::CamelEntityBundle,
    cat::CatEntityBundle,
    chicken::ChickenEntityBundle,
    cod::CodEntityBundle,
    cow::CowEntityBundle,
    donkey::DonkeyEntityBundle,
    frog::FrogEntityBundle,
    glow_squid::GlowSquidEntityBundle,
    horse::HorseEntityBundle,
    mooshroom::MooshroomEntityBundle,
    mule::MuleEntityBundle,
    ocelot::OcelotEntityBundle,
    parrot::ParrotEntityBundle,
    pig::PigEntityBundle,
    pufferfish::PufferfishEntityBundle,
    rabbit::RabbitEntityBundle,
    salmon::SalmonEntityBundle,
    sheep::SheepEntityBundle,
    skeleton_horse::SkeletonHorseEntityBundle,
    sniffer::SnifferEntityBundle,
    snow_golem::SnowGolemEntityBundle,
    squid::SquidEntityBundle,
    strider::StriderEntityBundle,
    tadpole::TadpoleEntityBundle,
    tropical_fish::TropicalFishEntityBundle,
    turtle::TurtleEntityBundle,
    villager::VillagerEntityBundle,
    wandering_trader::WanderingTraderEntityBundle,
    zombie_horse::ZombieHorseEntityBundle,
};

use crate::block_update::{handle_block_update, BlockUpdate, BlockUpdateEvent};

pub struct Building;

impl Plugin for Building {
    fn build(&self, app: &mut App) {
        app.insert_resource(Events::<BlockUpdateEvent>::default())
            .add_event::<CancelDiggingEvent>()
            .add_systems(Update, (digging, building, summoning).before(handle_block_update))
            .add_plugins(BlockUpdate);
    }
}

#[derive(Event, Debug, Clone)]
pub struct CancelDiggingEvent {
    pub client: Entity,
}

pub fn digging(
    clients: Query<(&GameMode, &EntityLayerId)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<DiggingEvent>,
    mut cancelled: EventReader<CancelDiggingEvent>,
    mut block_updates: EventWriter<BlockUpdateEvent>,
) {
    let cancelled = cancelled.read().map(|e| e.client).collect::<Vec<_>>();

    for event in events.read() {
        if cancelled.contains(&event.client) {
            continue;
        }

        let Ok((game_mode, &layer_id)) = clients.get(event.client) else {
            continue;
        };
        let Ok(mut layer) = layers.get_mut(layer_id.0) else {
            continue;
        };
        if (*game_mode == GameMode::Creative && event.state == DiggingState::Start)
            || (*game_mode == GameMode::Survival && event.state == DiggingState::Stop)
        {
            layer.set_block(event.position, BlockState::AIR);

            block_updates.send(BlockUpdateEvent {
                position: event.position,
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::Up),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::Down),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::North),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::East),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::South),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
            block_updates.send(BlockUpdateEvent {
                position: event.position.get_in_direction(Direction::West),
                layer: layer_id.0,
                entity_layer: layer_id,
            });
        }
    }
}

fn building(
    mut clients: Query<(&mut Inventory, &GameMode, &HeldItem, &Look, &Hitbox, &Flags, &EntityLayerId)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
    mut block_updates: EventWriter<BlockUpdateEvent>,
) {
    for event in events.read() {
        let Ok((mut inventory, game_mode, held, look, hitbox, flags, &layer_id)) = clients.get_mut(event.client)
        else {
            continue;
        };

        let Ok(mut layer) = layers.get_mut(layer_id.0) else {
            continue;
        };

        if event.head_inside_block {
            continue;
        }

        // get the held item
        let slot_id = held.slot();
        let stack = inventory.slot(slot_id);
        if stack.is_empty() {
            // no item in the slot
            continue;
        };

        let Some(block_kind) = BlockKind::from_item_kind(stack.item) else {
            // can't place this item as a block
            continue;
        };

        let (mut real_pos, replaceable) = if layer
            .block(event.position)
            .is_some_and(|b| !b.state.is_replaceable() || b.state.to_kind() == block_kind)
        {
            (event.position.get_in_direction(event.face), false)
        } else {
            (event.position, true)
        };
        let mut force_replace = false;
        let mut state = block_kind.to_state();
        if state.get(PropName::Axis).is_some() {
            state = state.set(
                PropName::Axis,
                match event.face {
                    Direction::Down | Direction::Up => PropValue::Y,
                    Direction::North | Direction::South => PropValue::Z,
                    Direction::West | Direction::East => PropValue::X,
                },
            );
        } else if state.get(PropName::Type).is_some() && state.get(PropName::Facing).is_none() {
            if let Some(half) = layer
                .block(event.position)
                .and_then(|b| {
                    b.state
                        .get(PropName::Type)
                        .filter(|_| b.state.to_kind() == state.to_kind())
                })
                .filter(|half| *half != PropValue::Double && !replaceable)
            {
                state = state.set(
                    PropName::Type,
                    match (half, event.face) {
                        (PropValue::Bottom, Direction::Up) => {
                            real_pos = event.position;
                            PropValue::Top
                        }
                        (PropValue::Top, Direction::Down) => {
                            real_pos = event.position;
                            PropValue::Bottom
                        }
                        (PropValue::Bottom, Direction::Down) => PropValue::Top,
                        (PropValue::Top, Direction::Up) => PropValue::Bottom,
                        _ => half,
                    },
                );
            } else {
                state = state.set(
                    PropName::Type,
                    match event.face {
                        Direction::Down => PropValue::Top,
                        Direction::Up => PropValue::Bottom,
                        _ if event.cursor_pos.y >= 0.5 => PropValue::Top,
                        _ => PropValue::Bottom,
                    },
                );
            }
            match layer
                .block(real_pos)
                .and_then(|b| b.state.get(PropName::Type))
                .zip(state.get(PropName::Type))
            {
                Some((PropValue::Top, PropValue::Bottom))
                | Some((PropValue::Bottom, PropValue::Top))
                    if layer
                        .block(real_pos)
                        .is_some_and(|b| b.state.to_kind() == state.to_kind()) =>
                {
                    force_replace = true;
                    state = state.set(PropName::Type, PropValue::Double);
                }
                _ => {}
            }
        } else if state.get(PropName::Facing).is_some()
            && state.get(PropName::Half).is_some()
            && state.get(PropName::Hinge).is_none()
        {
            state = state
                .set(
                    PropName::Facing,
                    match (look.yaw.floor() as i32).rem_euclid(360) {
                        45..135 => PropValue::West,
                        135..225 => PropValue::North,
                        225..315 => PropValue::East,
                        _ => PropValue::South,
                    },
                )
                .set(
                    PropName::Half,
                    match event.face {
                        Direction::Up => PropValue::Bottom,
                        Direction::Down => PropValue::Top,
                        _ if event.cursor_pos.y >= 0.5 => PropValue::Top,
                        _ => PropValue::Bottom,
                    },
                );
        } else if state.to_kind() == BlockKind::Anvil {
            state = state.set(
                PropName::Facing,
                match (look.yaw.floor() as i32).rem_euclid(360) {
                    45..135 => PropValue::North,
                    135..225 => PropValue::East,
                    225..315 => PropValue::South,
                    _ => PropValue::West,
                },
            );
        } else if state.to_kind() == BlockKind::AmethystCluster
            || state.to_kind() == BlockKind::SmallAmethystBud
            || state.to_kind() == BlockKind::MediumAmethystBud
            || state.to_kind() == BlockKind::LargeAmethystBud
            || state.to_kind() == BlockKind::Observer
        {
            state = state.set(
                PropName::Facing,
                match event.face {
                    Direction::Up => PropValue::Up,
                    Direction::Down => PropValue::Down,
                    Direction::North => PropValue::North,
                    Direction::East => PropValue::East,
                    Direction::South => PropValue::South,
                    Direction::West => PropValue::West,
                },
            );
        } else if state.get(PropName::Rotation).is_some() {
            state = state.set(
                PropName::Rotation,
                match look.yaw.rem_euclid(360.) {
                    11.25..33.75 => PropValue::_1,
                    33.75..56.25 => PropValue::_2,
                    56.25..78.75 => PropValue::_3,
                    78.75..101.25 => PropValue::_4,
                    101.25..123.75 => PropValue::_5,
                    123.75..146.25 => PropValue::_6,
                    146.25..168.75 => PropValue::_7,
                    168.75..191.25 => PropValue::_8,
                    191.25..213.75 => PropValue::_9,
                    213.75..236.25 => PropValue::_10,
                    236.25..258.75 => PropValue::_11,
                    258.75..281.25 => PropValue::_12,
                    281.25..303.75 => PropValue::_13,
                    303.75..326.25 => PropValue::_14,
                    326.25..348.75 => PropValue::_15,
                    _ => PropValue::_0,
                },
            );
        } else if state.to_kind() == BlockKind::WhiteWallBanner
            || state.to_kind() == BlockKind::LightGrayWallBanner
            || state.to_kind() == BlockKind::GrayWallBanner
            || state.to_kind() == BlockKind::BlackWallBanner
            || state.to_kind() == BlockKind::BrownWallBanner
            || state.to_kind() == BlockKind::RedWallBanner
            || state.to_kind() == BlockKind::OrangeWallBanner
            || state.to_kind() == BlockKind::YellowWallBanner
            || state.to_kind() == BlockKind::LimeWallBanner
            || state.to_kind() == BlockKind::GreenWallBanner
            || state.to_kind() == BlockKind::CyanWallBanner
            || state.to_kind() == BlockKind::LightBlueWallBanner
            || state.to_kind() == BlockKind::BlueWallBanner
            || state.to_kind() == BlockKind::PurpleWallBanner
            || state.to_kind() == BlockKind::MagentaWallBanner
            || state.to_kind() == BlockKind::PinkWallBanner
            || state.to_kind() == BlockKind::OakWallHangingSign
            || state.to_kind() == BlockKind::SpruceWallHangingSign
            || state.to_kind() == BlockKind::BirchWallHangingSign
            || state.to_kind() == BlockKind::JungleWallHangingSign
            || state.to_kind() == BlockKind::AcaciaWallHangingSign
            || state.to_kind() == BlockKind::DarkOakWallHangingSign
            || state.to_kind() == BlockKind::CrimsonWallHangingSign
            || state.to_kind() == BlockKind::CherryWallHangingSign
            || state.to_kind() == BlockKind::BambooWallHangingSign
            || state.to_kind() == BlockKind::MangroveWallHangingSign
            || state.to_kind() == BlockKind::WarpedWallHangingSign
            || state.to_kind() == BlockKind::Ladder
            || state.to_kind() == BlockKind::PinkPetals
            || state.to_kind() == BlockKind::RedstoneWallTorch
            || state.to_kind() == BlockKind::OakWallSign
            || state.to_kind() == BlockKind::SpruceWallSign
            || state.to_kind() == BlockKind::BirchWallSign
            || state.to_kind() == BlockKind::JungleWallSign
            || state.to_kind() == BlockKind::AcaciaWallSign
            || state.to_kind() == BlockKind::DarkOakWallSign
            || state.to_kind() == BlockKind::MangroveWallSign
            || state.to_kind() == BlockKind::CherryWallSign
            || state.to_kind() == BlockKind::BambooWallSign
            || state.to_kind() == BlockKind::CrimsonWallSign
            || state.to_kind() == BlockKind::WarpedWallSign
            || state.to_kind() == BlockKind::WallTorch
            || state.to_kind() == BlockKind::SoulWallTorch
            || state.to_kind() == BlockKind::TripwireHook
        {
            state = state.set(
                PropName::Facing,
                match event.face {
                    Direction::North => PropValue::North,
                    Direction::East => PropValue::East,
                    Direction::South => PropValue::South,
                    _ => PropValue::West,
                },
            );
        } else if state.to_kind() == BlockKind::Barrel {
            state = state.set(
                PropName::Facing,
                match (look.pitch, look.yaw.rem_euclid(360.)) {
                    (..-45.0, _) => PropValue::Down,
                    (45.0.., _) => PropValue::Up,
                    (_, 45.0..135.0) => PropValue::East,
                    (_, 135.0..225.0) => PropValue::South,
                    (_, 225.0..315.0) => PropValue::West,
                    _ => PropValue::North,
                },
            );
        } else if state.to_kind() == BlockKind::WhiteBed
            || state.to_kind() == BlockKind::LightGrayBed
            || state.to_kind() == BlockKind::GrayBed
            || state.to_kind() == BlockKind::BlackBed
            || state.to_kind() == BlockKind::BrownBed
            || state.to_kind() == BlockKind::RedBed
            || state.to_kind() == BlockKind::OrangeBed
            || state.to_kind() == BlockKind::YellowBed
            || state.to_kind() == BlockKind::LimeBed
            || state.to_kind() == BlockKind::GreenBed
            || state.to_kind() == BlockKind::CyanBed
            || state.to_kind() == BlockKind::LightBlueBed
            || state.to_kind() == BlockKind::BlueBed
            || state.to_kind() == BlockKind::PurpleBed
            || state.to_kind() == BlockKind::MagentaBed
            || state.to_kind() == BlockKind::PinkBed
        {
            let (dir, facing) = match (look.yaw.floor() as i32).rem_euclid(360) {
                45..135 => (Direction::West, PropValue::West),
                135..225 => (Direction::North, PropValue::North),
                225..315 => (Direction::East, PropValue::East),
                _ => (Direction::South, PropValue::South),
            };
            state = state.set(PropName::Facing, facing);
            if !layer
                .block(real_pos.get_in_direction(dir))
                .is_some_and(|b| b.state.is_replaceable())
            {
                continue;
            }
            if state.set(PropName::Part, PropValue::Head).collision_shapes().any(|c| (c + DVec3::new(real_pos.get_in_direction(dir).x as f64, real_pos.get_in_direction(dir).y as f64, real_pos.get_in_direction(dir).z as f64)).intersects(hitbox.get() + 0.5 * DVec3::Y)) {
                continue;
            }
            layer.set_block(
                real_pos.get_in_direction(dir),
                state.set(PropName::Part, PropValue::Head),
            );
        } else if state.to_kind() == BlockKind::Beehive
            || state.to_kind() == BlockKind::BeeNest
            || state.to_kind() == BlockKind::BigDripleaf
            || state.to_kind() == BlockKind::BlastFurnace
            || state.to_kind() == BlockKind::Campfire
            || state.to_kind() == BlockKind::EnderChest
            || state.to_kind() == BlockKind::ChiseledBookshelf
            || state.to_kind() == BlockKind::DecoratedPot
            || state.to_kind() == BlockKind::EndPortalFrame
            || state.to_kind() == BlockKind::Furnace
            || state.to_kind() == BlockKind::WhiteGlazedTerracotta
            || state.to_kind() == BlockKind::LightGrayGlazedTerracotta
            || state.to_kind() == BlockKind::GrayGlazedTerracotta
            || state.to_kind() == BlockKind::BlackGlazedTerracotta
            || state.to_kind() == BlockKind::BrownGlazedTerracotta
            || state.to_kind() == BlockKind::RedGlazedTerracotta
            || state.to_kind() == BlockKind::OrangeGlazedTerracotta
            || state.to_kind() == BlockKind::YellowGlazedTerracotta
            || state.to_kind() == BlockKind::LimeGlazedTerracotta
            || state.to_kind() == BlockKind::GreenGlazedTerracotta
            || state.to_kind() == BlockKind::CyanGlazedTerracotta
            || state.to_kind() == BlockKind::LightBlueGlazedTerracotta
            || state.to_kind() == BlockKind::BlueGlazedTerracotta
            || state.to_kind() == BlockKind::PurpleGlazedTerracotta
            || state.to_kind() == BlockKind::MagentaGlazedTerracotta
            || state.to_kind() == BlockKind::PinkGlazedTerracotta
            || state.to_kind() == BlockKind::JackOLantern
            || state.to_kind() == BlockKind::Lectern
            || state.to_kind() == BlockKind::Loom
            || state.to_kind() == BlockKind::Pumpkin
            || state.to_kind() == BlockKind::CarvedPumpkin
            || state.to_kind() == BlockKind::Smoker
            || state.to_kind() == BlockKind::Stonecutter
        {
            state = state.set(
                PropName::Facing,
                match (look.yaw.floor() as i32).rem_euclid(360) {
                    45..135 => PropValue::East,
                    135..225 => PropValue::South,
                    225..315 => PropValue::West,
                    _ => PropValue::North,
                },
            );
        } else if state.to_kind() == BlockKind::Bell {
            state = state
                .set(
                    PropName::Attachment,
                    match event.face {
                        Direction::Down => PropValue::Ceiling,
                        _ => PropValue::Floor,
                    },
                )
                .set(
                    PropName::Facing,
                    match (look.yaw.floor() as i32).rem_euclid(360) {
                        45..135 => PropValue::East,
                        135..225 => PropValue::South,
                        225..315 => PropValue::West,
                        _ => PropValue::North,
                    },
                );
        } else if state.to_kind() == BlockKind::OakButton
            || state.to_kind() == BlockKind::StoneButton
            || state.to_kind() == BlockKind::SpruceButton
            || state.to_kind() == BlockKind::JungleButton
            || state.to_kind() == BlockKind::AcaciaButton
            || state.to_kind() == BlockKind::CherryButton
            || state.to_kind() == BlockKind::WarpedButton
            || state.to_kind() == BlockKind::CrimsonButton
            || state.to_kind() == BlockKind::MangroveButton
            || state.to_kind() == BlockKind::DarkOakButton
            || state.to_kind() == BlockKind::BirchButton
            || state.to_kind() == BlockKind::BambooButton
            || state.to_kind() == BlockKind::PolishedBlackstoneButton
            || state.to_kind() == BlockKind::Grindstone
            || state.to_kind() == BlockKind::Lever
        {
            let face = match event.face {
                Direction::Up => PropValue::Floor,
                Direction::Down => PropValue::Ceiling,
                _ => PropValue::Wall,
            };
            state = state.set(PropName::Face, face).set(
                PropName::Facing,
                match face {
                    PropValue::Floor | PropValue::Ceiling => {
                        match (look.yaw.floor() as i32).rem_euclid(360) {
                            45..135 => PropValue::West,
                            135..225 => PropValue::North,
                            225..315 => PropValue::East,
                            _ => PropValue::South,
                        }
                    }
                    _ => match event.face {
                        Direction::North => PropValue::North,
                        Direction::East => PropValue::East,
                        Direction::South => PropValue::South,
                        _ => PropValue::West,
                    },
                },
            );
        } else if state.to_kind() == BlockKind::Chest || state.to_kind() == BlockKind::TrappedChest
        {
            let facing = match (look.yaw.floor() as i32).rem_euclid(360) {
                45..135 => PropValue::East,
                135..225 => PropValue::South,
                225..315 => PropValue::West,
                _ => PropValue::North,
            };
            state = state.set(PropName::Facing, facing);
            let (left, right) = match facing {
                PropValue::North => (Direction::East, Direction::West),
                PropValue::East => (Direction::South, Direction::North),
                PropValue::South => (Direction::West, Direction::East),
                _ => (Direction::North, Direction::South),
            };
            if let Some(adjoint_facing) = layer
                .block(event.position)
                .filter(|b| b.state.to_kind() == state.to_kind())
                .and_then(|b| b.state.get(PropName::Facing))
            {
                state = state.set(
                    PropName::Type,
                    match (adjoint_facing, event.face) {
                        (PropValue::North, Direction::East)
                        | (PropValue::East, Direction::South)
                        | (PropValue::South, Direction::West)
                        | (PropValue::West, Direction::North) => {
                            layer.set_block(
                                real_pos.get_in_direction(right),
                                state.set(PropName::Type, PropValue::Left),
                            );
                            PropValue::Right
                        }
                        (PropValue::North, Direction::West)
                        | (PropValue::East, Direction::North)
                        | (PropValue::South, Direction::East)
                        | (PropValue::West, Direction::South) => {
                            layer.set_block(
                                real_pos.get_in_direction(left),
                                state.set(PropName::Type, PropValue::Right),
                            );
                            PropValue::Left
                        }
                        _ => PropValue::Single,
                    },
                );
                if state.get(PropName::Type) != Some(PropValue::Single) {
                    state = state.set(PropName::Facing, adjoint_facing);
                }
            } else if layer
                .block(real_pos.get_in_direction(left))
                .filter(|b| b.state.to_kind() == state.to_kind())
                .and_then(|b| {
                    b.state
                        .get(PropName::Facing)
                        .zip(b.state.get(PropName::Type))
                })
                .is_some_and(|(f, t)| f == facing && t == PropValue::Single && !flags.sneaking())
            {
                layer.set_block(
                    real_pos.get_in_direction(left),
                    state.set(PropName::Type, PropValue::Right),
                );
                state = state.set(PropName::Type, PropValue::Left);
            } else if layer
                .block(real_pos.get_in_direction(right))
                .filter(|b| b.state.to_kind() == state.to_kind())
                .and_then(|b| {
                    b.state
                        .get(PropName::Facing)
                        .zip(b.state.get(PropName::Type))
                })
                .is_some_and(|(f, t)| f == facing && t == PropValue::Single && !flags.sneaking())
            {
                layer.set_block(
                    real_pos.get_in_direction(right),
                    state.set(PropName::Type, PropValue::Left),
                );
                state = state.set(PropName::Type, PropValue::Right);
            }
        } else if state.to_kind() == BlockKind::Cocoa {
            state = state.set(
                PropName::Facing,
                match event.face {
                    Direction::Up | Direction::Down => {
                        match (look.yaw.floor() as i32).rem_euclid(360) {
                            45..135 => PropValue::West,
                            135..225 => PropValue::North,
                            225..315 => PropValue::East,
                            _ => PropValue::South,
                        }
                    }
                    Direction::North => PropValue::South,
                    Direction::East => PropValue::West,
                    Direction::South => PropValue::North,
                    _ => PropValue::East,
                },
            );
        } else if state.to_kind() == BlockKind::CommandBlock
            || state.to_kind() == BlockKind::RepeatingCommandBlock
            || state.to_kind() == BlockKind::ChainCommandBlock
            || state.to_kind() == BlockKind::EndRod
            || state.to_kind() == BlockKind::LightningRod
        {
            state = state.set(
                PropName::Facing,
                match event.face {
                    Direction::Up => PropValue::Up,
                    Direction::Down => PropValue::Down,
                    Direction::North => PropValue::North,
                    Direction::East => PropValue::East,
                    Direction::South => PropValue::South,
                    Direction::West => PropValue::West,
                },
            );
        } else if state.to_kind() == BlockKind::TubeCoralWallFan
            || state.to_kind() == BlockKind::BrainCoralWallFan
            || state.to_kind() == BlockKind::BubbleCoralWallFan
            || state.to_kind() == BlockKind::FireCoralWallFan
            || state.to_kind() == BlockKind::HornCoralWallFan
        {
            state = state.set(
                PropName::Facing,
                match event.face {
                    Direction::North => PropValue::North,
                    Direction::East => PropValue::East,
                    Direction::South => PropValue::South,
                    _ => PropValue::West,
                },
            );
        } else if state.to_kind() == BlockKind::Dispenser
            || state.to_kind() == BlockKind::Dropper
            || state.to_kind() == BlockKind::ShulkerBox
        {
            state = state.set(
                PropName::Facing,
                match event.face {
                    Direction::Up => PropValue::Down,
                    Direction::Down => PropValue::Up,
                    Direction::North => PropValue::South,
                    Direction::East => PropValue::West,
                    Direction::South => PropValue::North,
                    Direction::West => PropValue::East,
                },
            );
        } else if state.get(PropName::Hinge).is_some() {
            if !layer
                .block(real_pos.get_in_direction(Direction::Down))
                .is_some_and(|b| b.state.is_opaque())
            {
                continue;
            }
            let facing = match (look.yaw.floor() as i32).rem_euclid(360) {
                45..135 => PropValue::West,
                135..225 => PropValue::North,
                225..315 => PropValue::East,
                _ => PropValue::South,
            };
            state = state.set(PropName::Facing, facing);
            let (left, right) = match facing {
                PropValue::North => (Direction::East, Direction::West),
                PropValue::East => (Direction::South, Direction::North),
                PropValue::South => (Direction::West, Direction::East),
                _ => (Direction::North, Direction::South),
            };
            let matching_left = layer
                .block(real_pos.get_in_direction(left))
                .is_some_and(|b| {
                    b.state.to_kind() == state.to_kind()
                        && b.state.get(PropName::Hinge) == Some(PropValue::Right)
                });
            let matching_right = layer
                .block(real_pos.get_in_direction(right))
                .is_some_and(|b| {
                    b.state.to_kind() == state.to_kind()
                        && b.state.get(PropName::Hinge) == Some(PropValue::Left)
                });
            if matching_left && !matching_right {
                state = state.set(PropName::Hinge, PropValue::Left);
            } else if matching_right && !matching_left {
                state = state.set(PropName::Hinge, PropValue::Right);
            } else {
                let left_blocks = layer
                    .block(real_pos.get_in_direction(left))
                    .is_some_and(|b| b.state.is_opaque()) as u8
                    + layer
                        .block(
                            real_pos
                                .get_in_direction(left)
                                .get_in_direction(Direction::Up),
                        )
                        .is_some_and(|b| b.state.is_opaque()) as u8;
                let right_blocks = layer
                    .block(real_pos.get_in_direction(right))
                    .is_some_and(|b| b.state.is_opaque()) as u8
                    + layer
                        .block(
                            real_pos
                                .get_in_direction(right)
                                .get_in_direction(Direction::Up),
                        )
                        .is_some_and(|b| b.state.is_opaque()) as u8;
                let aim = match facing {
                    PropValue::North => event.cursor_pos.x,
                    PropValue::East => event.cursor_pos.z,
                    PropValue::South => 1. - event.cursor_pos.x,
                    _ => 1. - event.cursor_pos.z,
                };
                state = state.set(
                    PropName::Hinge,
                    if left_blocks > right_blocks {
                        PropValue::Left
                    } else if right_blocks > left_blocks || aim > 0.5 {
                        PropValue::Right
                    } else {
                        PropValue::Left
                    },
                );
            }
            if layer
                .block(real_pos.get_in_direction(Direction::Up))
                .filter(|b| !b.state.is_replaceable())
                .is_none()
            {
                if state.set(PropName::Half, PropValue::Upper).collision_shapes().any(|c| (c + DVec3::new(real_pos.x as f64, (real_pos.y + 1) as f64, real_pos.z as f64)).intersects(hitbox.get() + 0.5 * DVec3::Y)) {
                    continue;
                }
                layer.set_block(
                    real_pos.get_in_direction(Direction::Up),
                    state.set(PropName::Half, PropValue::Upper),
                );
            } else {
                continue;
            }
        } else if state.to_kind() == BlockKind::LargeFern
            || state.to_kind() == BlockKind::TallGrass
            || state.to_kind() == BlockKind::Lilac
            || state.to_kind() == BlockKind::Peony
            || state.to_kind() == BlockKind::PitcherPlant
            || state.to_kind() == BlockKind::RoseBush
            || state.to_kind() == BlockKind::Sunflower
        {
            if !layer
                .block(real_pos.get_in_direction(Direction::Down))
                .is_some_and(|b| b.state.is_opaque())
            {
                continue;
            }
            if layer
                .block(real_pos.get_in_direction(Direction::Up))
                .filter(|b| !b.state.is_replaceable())
                .is_none()
            {
                layer.set_block(
                    real_pos.get_in_direction(Direction::Up),
                    state.set(PropName::Half, PropValue::Upper),
                );
            } else {
                continue;
            }
        } else if state.to_kind() == BlockKind::GlowLichen {
            let Some(original_block) = layer.block(real_pos) else {
                continue;
            };
            match event.face {
                Direction::Up
                    if original_block.state.get(PropName::Down) != Some(PropValue::True) =>
                {
                    state = original_block.state.set(PropName::Down, PropValue::True);
                }
                Direction::Down
                    if original_block.state.get(PropName::Up) != Some(PropValue::True) =>
                {
                    state = original_block.state.set(PropName::Down, PropValue::True);
                }
                Direction::North
                    if original_block.state.get(PropName::South) != Some(PropValue::True) =>
                {
                    state = original_block.state.set(PropName::Down, PropValue::True);
                }
                Direction::East
                    if original_block.state.get(PropName::West) != Some(PropValue::True) =>
                {
                    state = original_block.state.set(PropName::Down, PropValue::True);
                }
                Direction::South
                    if original_block.state.get(PropName::North) != Some(PropValue::True) =>
                {
                    state = original_block.state.set(PropName::Down, PropValue::True);
                }
                Direction::West
                    if original_block.state.get(PropName::East) != Some(PropValue::True) =>
                {
                    state = original_block.state.set(PropName::Down, PropValue::True);
                }
                _ => {
                    continue;
                }
            }
            if original_block.state.to_kind() == BlockKind::GlowLichen {
                force_replace = true;
            }
        } else if state.to_kind() == BlockKind::Hopper {
            state = state.set(
                PropName::Facing,
                match event.face {
                    Direction::Up | Direction::Down => PropValue::Down,
                    Direction::North => PropValue::South,
                    Direction::East => PropValue::West,
                    Direction::South => PropValue::North,
                    Direction::West => PropValue::East,
                },
            );
        } else if state.to_kind() == BlockKind::Lantern || state.to_kind() == BlockKind::SoulLantern
        {
            if event.face == Direction::Down {
                state = state.set(PropName::Hanging, PropValue::True);
            }
        } else if state.get(PropName::Persistent).is_some() {
            state = state.set(PropName::Persistent, PropValue::True);
        } else if state.to_kind() == BlockKind::Piston || state.to_kind() == BlockKind::StickyPiston
        {
            state = state.set(
                PropName::Facing,
                match event.face {
                    Direction::Up => PropValue::Down,
                    Direction::Down => PropValue::Up,
                    Direction::North => PropValue::South,
                    Direction::East => PropValue::West,
                    Direction::South => PropValue::North,
                    Direction::West => PropValue::East,
                },
            );
        } else if state.to_kind() == BlockKind::Rail
            || state.to_kind() == BlockKind::ActivatorRail
            || state.to_kind() == BlockKind::DetectorRail
            || state.to_kind() == BlockKind::PoweredRail
        {
            if !layer
                .block(real_pos.get_in_direction(Direction::Down))
                .is_some_and(|b| b.state.is_opaque())
            {
                continue;
            }
            let facing = match (look.yaw.floor() as i32).rem_euclid(360) {
                45..135 | 225..315 => PropValue::EastWest,
                _ => PropValue::NorthSouth,
            };
            state = state.set(PropName::Shape, facing);
        } else if state.to_kind() == BlockKind::Comparator || state.to_kind() == BlockKind::Repeater
        {
            if !layer
                .block(real_pos.get_in_direction(Direction::Down))
                .is_some_and(|b| b.state.is_opaque())
            {
                continue;
            }
            state = state.set(
                PropName::Facing,
                match (look.yaw.floor() as i32).rem_euclid(360) {
                    45..135 => PropValue::East,
                    135..225 => PropValue::South,
                    225..315 => PropValue::West,
                    _ => PropValue::North,
                },
            );
        } else if (state.to_kind() == BlockKind::RedstoneTorch
            || state.to_kind() == BlockKind::Torch
            || state.to_kind() == BlockKind::SoulTorch)
            && !layer
                .block(real_pos.get_in_direction(Direction::Down))
                .is_some_and(|b| b.state.is_opaque())
        {
            continue;
        } else if state.to_kind() == BlockKind::SmallDripleaf {
            state = state.set(
                PropName::Facing,
                match (look.yaw.floor() as i32).rem_euclid(360) {
                    45..135 => PropValue::East,
                    135..225 => PropValue::South,
                    225..315 => PropValue::West,
                    _ => PropValue::North,
                },
            );
            if layer
                .block(real_pos.get_in_direction(Direction::Up))
                .filter(|b| !b.state.is_replaceable())
                .is_none()
            {
                layer.set_block(
                    real_pos.get_in_direction(Direction::Up),
                    state.set(PropName::Half, PropValue::Upper),
                );
            } else {
                continue;
            }
        } else if state.to_kind() == BlockKind::Snow {
            if let Some(layers) = layer
                .block(event.position)
                .filter(|b| b.state.to_kind() == BlockKind::Snow && event.face == Direction::Up)
                .and_then(|b| b.state.get(PropName::Layers))
            {
                state = state.set(
                    PropName::Layers,
                    match layers {
                        PropValue::_0 => PropValue::_1,
                        PropValue::_1 => PropValue::_2,
                        PropValue::_2 => PropValue::_3,
                        PropValue::_3 => PropValue::_4,
                        PropValue::_4 => PropValue::_5,
                        PropValue::_5 => PropValue::_6,
                        _ => PropValue::_7,
                    },
                );
                if layers == PropValue::_7 {
                    state = BlockState::SNOW_BLOCK;
                }
                real_pos = event.position;
                force_replace = true;
            } else if let Some(layers) = layer
                .block(real_pos)
                .filter(|b| b.state.to_kind() == BlockKind::Snow)
                .and_then(|b| b.state.get(PropName::Layers))
            {
                state = state.set(
                    PropName::Layers,
                    match layers {
                        PropValue::_0 => PropValue::_1,
                        PropValue::_1 => PropValue::_2,
                        PropValue::_2 => PropValue::_3,
                        PropValue::_3 => PropValue::_4,
                        PropValue::_4 => PropValue::_5,
                        PropValue::_5 => PropValue::_6,
                        _ => PropValue::_7,
                    },
                );
                if layers == PropValue::_7 {
                    state = BlockState::SNOW_BLOCK;
                }
                force_replace = true;
            }
            if !layer
                .block(real_pos.get_in_direction(Direction::Down))
                .is_some_and(|b| b.state.is_opaque())
            {
                continue;
            }
        } else if state.to_kind() == BlockKind::TallSeagrass {
            if !layer
                .block(real_pos)
                .is_some_and(|b| b.state.to_kind() == BlockKind::Water)
                || !layer
                    .block(real_pos.get_in_direction(Direction::Up))
                    .is_some_and(|b| b.state.to_kind() == BlockKind::Water)
            {
                continue;
            }
            layer.set_block(
                real_pos.get_in_direction(Direction::Up),
                state.set(PropName::Half, PropValue::Upper),
            );
        } else if state.to_kind() == BlockKind::OakTrapdoor
            || state.to_kind() == BlockKind::SpruceTrapdoor
            || state.to_kind() == BlockKind::BirchTrapdoor
            || state.to_kind() == BlockKind::JungleTrapdoor
            || state.to_kind() == BlockKind::AcaciaTrapdoor
            || state.to_kind() == BlockKind::DarkOakTrapdoor
            || state.to_kind() == BlockKind::MangroveTrapdoor
            || state.to_kind() == BlockKind::CherryTrapdoor
            || state.to_kind() == BlockKind::BambooTrapdoor
            || state.to_kind() == BlockKind::CrimsonTrapdoor
            || state.to_kind() == BlockKind::WarpedTrapdoor
            || state.to_kind() == BlockKind::IronTrapdoor
        {
            state = state
                .set(
                    PropName::Facing,
                    match event.face {
                        Direction::Up | Direction::Down => {
                            match (look.yaw.floor() as i32).rem_euclid(360) {
                                45..135 => PropValue::West,
                                135..225 => PropValue::North,
                                225..315 => PropValue::East,
                                _ => PropValue::South,
                            }
                        }
                        _ => match event.face {
                            Direction::North => PropValue::North,
                            Direction::East => PropValue::East,
                            Direction::South => PropValue::South,
                            _ => PropValue::West,
                        },
                    },
                )
                .set(
                    PropName::Type,
                    match event.face {
                        Direction::Down => PropValue::Top,
                        Direction::Up => PropValue::Bottom,
                        _ if event.cursor_pos.y >= 0.5 => PropValue::Top,
                        _ => PropValue::Bottom,
                    },
                );
        } else if state.to_kind() == BlockKind::Vine {
            if !layer
                .block(event.position)
                .is_some_and(|b| b.state.is_opaque())
            {
                continue;
            }
            if let Some(original_state) = layer
                .block(real_pos)
                .filter(|b| b.state.to_kind() == BlockKind::Vine)
                .map(|b| b.state)
            {
                match event.face {
                    Direction::North
                        if original_state.get(PropName::South) == Some(PropValue::False) =>
                    {
                        state = original_state.set(PropName::South, PropValue::True);
                    }
                    Direction::East
                        if original_state.get(PropName::West) == Some(PropValue::False) =>
                    {
                        state = original_state.set(PropName::West, PropValue::True);
                    }
                    Direction::South
                        if original_state.get(PropName::North) == Some(PropValue::False) =>
                    {
                        state = original_state.set(PropName::North, PropValue::True);
                    }
                    Direction::West
                        if original_state.get(PropName::East) == Some(PropValue::False) =>
                    {
                        state = original_state.set(PropName::East, PropValue::True);
                    }
                    Direction::Down
                        if original_state.get(PropName::Up) == Some(PropValue::False) =>
                    {
                        state = original_state.set(PropName::Up, PropValue::True);
                    }
                    _ => {
                        continue;
                    }
                }
            } else {
                match event.face {
                    Direction::North => {
                        state = state.set(PropName::South, PropValue::True);
                    }
                    Direction::East => {
                        state = state.set(PropName::West, PropValue::True);
                    }
                    Direction::South => {
                        state = state.set(PropName::North, PropValue::True);
                    }
                    Direction::West => {
                        state = state.set(PropName::East, PropValue::True);
                    }
                    Direction::Down => {
                        state = state.set(PropName::Up, PropValue::True);
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }

        if state.collision_shapes().any(|c| (c + DVec3::new(real_pos.x as f64, real_pos.y as f64, real_pos.z as f64)).intersects(hitbox.get() + 0.5 * DVec3::Y)) {
            continue;
        }

        if *game_mode == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            if stack.count > 1 {
                let amount = stack.count - 1;
                inventory.set_slot_amount(slot_id, amount);
            } else {
                inventory.set_slot(slot_id, ItemStack::EMPTY);
            }
        }

        if layer
            .block(real_pos)
            .is_some_and(|b| b.state.to_kind() == BlockKind::Water)
            && state.get(PropName::Waterlogged).is_some()
        {
            state = state.set(PropName::Waterlogged, PropValue::True);
        }

        if !force_replace
            && layer
                .block(real_pos)
                .is_some_and(|b| !b.state.is_replaceable() || b.state.to_kind() == state.to_kind())
        {
            continue;
        }
        // client.send_chat_message(format!("{:?}", state));
        layer.set_block(real_pos, state);

        block_updates.send(BlockUpdateEvent { position: real_pos, layer: layer_id.0, entity_layer: layer_id });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::Up),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::Down),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::North),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::East),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::South),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
        block_updates.send(BlockUpdateEvent {
            position: real_pos.get_in_direction(Direction::West),
            layer: layer_id.0,
            entity_layer: layer_id,
        });
    }
}

fn summoning(
    mut clients: Query<(&mut Inventory, &GameMode, &HeldItem, &EntityLayerId)>,
    mut events: EventReader<InteractBlockEvent>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Ok((mut inventory, game_mode, held, &layer)) = clients.get_mut(event.client) else {
            continue;
        };

        let slot_id = held.slot();
        let stack = inventory.slot(slot_id);

        if stack.is_empty() {
            continue;
        }

        let real_pos = event.position.get_in_direction(event.face);
        let position = Position(DVec3::new(real_pos.x.into(), real_pos.y.into(), real_pos.z.into()));

        match stack.item {
            ItemKind::BlazeSpawnEgg => { commands.spawn(BlazeEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::CreeperSpawnEgg => { commands.spawn(CreeperEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ElderGuardianSpawnEgg => { commands.spawn(ElderGuardianEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::EnderDragonSpawnEgg => { commands.spawn(EnderDragonEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::EndermiteSpawnEgg => { commands.spawn(EndermiteEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::EvokerSpawnEgg => { commands.spawn(EvokerEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::GhastSpawnEgg => { commands.spawn(GhastEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::GuardianSpawnEgg => { commands.spawn(GuardianEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::HoglinSpawnEgg => { commands.spawn(HoglinEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::HuskSpawnEgg => { commands.spawn(HuskEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::MagmaCubeSpawnEgg => { commands.spawn(MagmaCubeEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::PhantomSpawnEgg => { commands.spawn(PhantomEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::PiglinBruteSpawnEgg => { commands.spawn(PiglinBruteEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::PillagerSpawnEgg => { commands.spawn(PillagerEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::RavagerSpawnEgg => { commands.spawn(RavagerEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ShulkerSpawnEgg => { commands.spawn(ShulkerEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SilverfishSpawnEgg => { commands.spawn(SilverfishEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SkeletonSpawnEgg => { commands.spawn(SkeletonEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SlimeSpawnEgg => { commands.spawn(SlimeEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::StraySpawnEgg => { commands.spawn(StrayEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::VexSpawnEgg => { commands.spawn(VexEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::VindicatorSpawnEgg => { commands.spawn(VindicatorEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::WardenSpawnEgg => { commands.spawn(WardenEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::WitchSpawnEgg => { commands.spawn(WitchEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::WitherSpawnEgg => { commands.spawn(WitherEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::WitherSkeletonSpawnEgg => { commands.spawn(WitherSkeletonEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ZoglinSpawnEgg => { commands.spawn(ZoglinEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ZombieSpawnEgg => { commands.spawn(ZombieEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ZombieVillagerSpawnEgg => { commands.spawn(ZombieVillagerEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::BeeSpawnEgg => { commands.spawn(BeeEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::CaveSpiderSpawnEgg => { commands.spawn(CaveSpiderEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::EndermanSpawnEgg => { commands.spawn(EndermanEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::DolphinSpawnEgg => { commands.spawn(DolphinEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::DrownedSpawnEgg => { commands.spawn(DrownedEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::FoxSpawnEgg => { commands.spawn(FoxEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::GoatSpawnEgg => { commands.spawn(GoatEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::IronGolemSpawnEgg => { commands.spawn(IronGolemEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::LlamaSpawnEgg => { commands.spawn(LlamaEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::PandaSpawnEgg => { commands.spawn(PandaEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::PiglinSpawnEgg => { commands.spawn(PiglinEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::PolarBearSpawnEgg => { commands.spawn(PolarBearEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SpiderSpawnEgg => { commands.spawn(SpiderEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::TraderLlamaSpawnEgg => { commands.spawn(TraderLlamaEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::WolfSpawnEgg => { commands.spawn(WolfEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ZombifiedPiglinSpawnEgg => { commands.spawn(ZombifiedPiglinEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::AllaySpawnEgg => { commands.spawn(AllayEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::AxolotlSpawnEgg => { commands.spawn(AxolotlEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::BatSpawnEgg => { commands.spawn(BatEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::CamelSpawnEgg => { commands.spawn(CamelEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::CatSpawnEgg => { commands.spawn(CatEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ChickenSpawnEgg => { commands.spawn(ChickenEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::CodSpawnEgg => { commands.spawn(CodEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::CowSpawnEgg => { commands.spawn(CowEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::DonkeySpawnEgg => { commands.spawn(DonkeyEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::FrogSpawnEgg => { commands.spawn(FrogEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::GlowSquidSpawnEgg => { commands.spawn(GlowSquidEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::HorseSpawnEgg => { commands.spawn(HorseEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::MooshroomSpawnEgg => { commands.spawn(MooshroomEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::MuleSpawnEgg => { commands.spawn(MuleEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::OcelotSpawnEgg => { commands.spawn(OcelotEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ParrotSpawnEgg => { commands.spawn(ParrotEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::PigSpawnEgg => { commands.spawn(PigEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::PufferfishSpawnEgg => { commands.spawn(PufferfishEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::RabbitSpawnEgg => { commands.spawn(RabbitEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SalmonSpawnEgg => { commands.spawn(SalmonEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SheepSpawnEgg => { commands.spawn(SheepEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SkeletonHorseSpawnEgg => { commands.spawn(SkeletonHorseEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SnifferSpawnEgg => { commands.spawn(SnifferEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SnowGolemSpawnEgg => { commands.spawn(SnowGolemEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::SquidSpawnEgg => { commands.spawn(SquidEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::StriderSpawnEgg => { commands.spawn(StriderEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::TadpoleSpawnEgg => { commands.spawn(TadpoleEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::TropicalFishSpawnEgg => { commands.spawn(TropicalFishEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::TurtleSpawnEgg => { commands.spawn(TurtleEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::VillagerSpawnEgg => { commands.spawn(VillagerEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::WanderingTraderSpawnEgg => { commands.spawn(WanderingTraderEntityBundle { position, layer, ..Default::default() }); }
            ItemKind::ZombieHorseSpawnEgg => { commands.spawn(ZombieHorseEntityBundle { position, layer, ..Default::default() }); }
            _ => {
                continue;
            }
        }

        if *game_mode == GameMode::Survival {
            // check if the player has the item in their inventory and remove
            // it.
            if stack.count > 1 {
                let amount = stack.count - 1;
                inventory.set_slot_amount(slot_id, amount);
            } else {
                inventory.set_slot(slot_id, ItemStack::EMPTY);
            }
        }
    }
}

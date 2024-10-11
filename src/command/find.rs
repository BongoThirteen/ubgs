
use std::cmp::Ordering;

use thiserror::Error;
use valence::command::Command;
use valence::prelude::*;
use valence::command::parsers::entity_selector::EntitySelectors;
use valence::math::Aabb;
use valence::rand::seq::{IteratorRandom, SliceRandom};
use valence::rand::thread_rng;
use valence::entity::living::LivingEntity;
use valence::scoreboard::{Objective, ObjectiveScores};
use valence::command::parsers::EntitySelector;
use valence::command::handler::CommandResultEvent;

use crate::players::Xp;

#[derive(Debug, Error)]
pub enum FindTargetError {
    #[error("`{0}` is not valid syntax for an entity selector argument")]
    ArgumentSyntax(String),
    #[error("`{0}` is not a valid number")]
    InvalidNumber(String),
    #[error("Could not find player with name `{0}`")]
    NameNotFound(String),
    #[error("Unknown selector argument `{0}`")]
    ArgumentNotFound(String),
    #[error("Selector argument `{0}` cannot be duplicated")]
    DuplicateArgument(String),
    #[error("There is no scoreboard objective with name `{0}`")]
    ObjectiveNotFound(String),
    #[error("`{0}` is not a valid sort order")]
    InvalidSort(String),
    #[error("`{0}` is not a valid gamemode")]
    InvalidGamemode(String),
    #[error("`{0}` is not a valid entity type")]
    InvalidType(String),
    #[error("Internal error: entity layer not found")]
    LayerNotFound,
    #[error("Internal error: position not found")]
    PositionNotFound,
    #[error("There are no other players around you. Nearest player does not select you.")]
    NearestNotFound,
    #[error("Internal error: apparently you don't exist")]
    RandomNotFound,
}

pub fn find_targets<C: Command + Send + Sync>(
    players: &Query<
        (
            Entity,
            &EntityLayerId,
            &Position,
            &Username,
            &Xp,
            &GameMode,
        ),
    >,
    living_entities: &Query<(Entity, &EntityLayerId, &Position, &EntityKind), With<LivingEntity>>,
    scoreboard: &Query<(&Objective, &ObjectiveScores)>,
    event: &CommandResultEvent<C>,
    target: &EntitySelector,
) -> Result<Vec<Entity>, FindTargetError> {

    let simple = match target {
        EntitySelector::SimpleSelector(simple) => simple,
        EntitySelector::ComplexSelector(simple, _) => simple,
    };
    let targets = match simple {
        EntitySelectors::AllEntities => {
            let Ok((_, EntityLayerId(executor_entity_layer), ..)) = living_entities.get(event.executor) else {
                return Err(FindTargetError::LayerNotFound);
            };
            let targets = living_entities
                .iter()
                .filter_map(|(entity, layer, ..)| (layer.0 == *executor_entity_layer).then_some(entity)).collect();
            Ok(targets)
        }
        EntitySelectors::SinglePlayer(name) => {
            let Some((target, ..)) = players.iter().find(|(_, _, _, username, ..)| username.0 == *name) else {
                return Err(FindTargetError::NameNotFound(name.clone()));
            };
            Ok(vec![target])
        }
        EntitySelectors::AllPlayers => {
            let Ok((_, EntityLayerId(executor_entity_layer), ..)) = players.get(event.executor) else {
                return Err(FindTargetError::LayerNotFound);
            };
            let targets = players.iter().filter_map(|(entity, layer, ..)| (layer == &EntityLayerId(*executor_entity_layer)).then_some(entity)).collect();
            Ok(targets)
        }
        EntitySelectors::SelfPlayer => {
            Ok(vec![event.executor])
        }
        EntitySelectors::NearestPlayer => {
            let Ok((_, EntityLayerId(executor_entity_layer), Position(executor_pos), ..)) = players.get(event.executor) else {
                return Err(FindTargetError::LayerNotFound);
            };
            let mut targets = players
                .iter()
                .filter(|(_, layer, ..)| *layer == &EntityLayerId(*executor_entity_layer))
                .filter(|(target, ..)| *target != event.executor)
                .map(|(target, ..)| target)
                .collect::<Vec<_>>();
            targets.sort_by(|target_a, target_b| {
                let Ok((dist_a, dist_b)) = 
                    players.get(*target_a).map(|(_, _, p, ..)| p.distance(*executor_pos)).and_then(|dist_a|
                        Ok((dist_a, players.get(*target_b).map(|(_, _, p, ..)| p.distance(*executor_pos))?))) else {
                    return Ordering::Equal;
                };
                dist_a.partial_cmp(&dist_b).unwrap_or(Ordering::Equal)
            });
            Ok(targets)
        }
        EntitySelectors::RandomPlayer => {
            let Ok((_, EntityLayerId(executor_entity_layer), ..)) = players.get(event.executor) else {
                return Err(FindTargetError::LayerNotFound);
            };
            let targets = players.iter().filter_map(|(entity, layer, ..)| (layer == &EntityLayerId(*executor_entity_layer)).then_some(entity)).collect();
            Ok(targets)
        }
    };
    if let EntitySelector::ComplexSelector(_, complex) = target {
        let mut arguments = Vec::new(); // complex.split(',').map(|arg| arg.split_once('=').ok_or_else(|| FindTargetError::ArgumentSyntax(arg.to_owned()))).collect::<Result<Vec<_>, _>>()?;
        let mut level = 0;
        let mut sub = 0..0;
        for (i, c) in complex.chars().enumerate() {
            if c == '{' {
                level += 1;
            } else if c == '}' {
                level -= 1;
            } else if level == 0 && c == ',' {
                arguments.push(&complex[sub]);
                sub = i..i;
            } else {
                sub.end = i + 1;
            }
        }
        let arguments = arguments.into_iter().map(|arg| arg.split_once('=').ok_or_else(|| FindTargetError::ArgumentSyntax(arg.to_owned()))).collect::<Result<Vec<_>, _>>()?;
        let mut targets = targets?;
        let mut x_pos = None;
        let mut y_pos = None;
        let mut z_pos = None;
        let mut dx = None;
        let mut dy = None;
        let mut dz = None;
        for (arg, val) in arguments {
            if arg == "x" {
                if x_pos.is_none() {
                    x_pos = Some(val.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?);
                } else {
                    return Err(FindTargetError::DuplicateArgument(arg.to_owned()));
                }
            } else if arg == "y" {
                if y_pos.is_none() {
                    y_pos = Some(val.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?);
                } else {
                    return Err(FindTargetError::DuplicateArgument(arg.to_owned()));
                }
            } else if arg == "z" {
                if z_pos.is_none() {
                    z_pos = Some(val.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?);
                } else {
                    return Err(FindTargetError::DuplicateArgument(arg.to_owned()));
                }
            } else if arg == "dx" {
                if dx.is_none() {
                    dx = Some(val.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?);
                } else {
                    return Err(FindTargetError::DuplicateArgument(arg.to_owned()));
                }
            } else if arg == "dy" {
                if dx.is_none() {
                    dy = Some(val.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?);
                } else {
                    return Err(FindTargetError::DuplicateArgument(arg.to_owned()));
                }
            } else if arg == "dz" {
                if dx.is_none() {
                    dz = Some(val.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?);
                } else {
                    return Err(FindTargetError::DuplicateArgument(arg.to_owned()));
                }
            } else if arg == "distance" {
                let pos = if let Ok((.., Position(executor_pos), _)) = living_entities.get(event.executor) {
                    *executor_pos
                } else if let (Some(x), Some(y), Some(z)) = (x_pos, y_pos, z_pos) {
                    DVec3::new(x, y, z)
                } else {
                    return Err(FindTargetError::PositionNotFound);
                };
                if let Some((min, max)) = val.split_once("..") {
                    let (min, max) = (
                        (!min.is_empty()).then_some(min.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?),
                        (!max.is_empty()).then_some(max.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?),
                    );
                    targets.retain(|e| living_entities.get(*e).is_ok_and(|(.., p, _)| (min..max).contains(&Some(p.distance(pos)))));
                } else {
                    let val = val.parse::<f64>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?;
                    targets.retain(|e| living_entities.get(*e).is_ok_and(|(.., p, _)| p.distance(pos) == val));
                }
            } else if arg == "limit" {
                targets.truncate(arg.parse::<usize>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?);
            } else if arg == "scores" {
                let scores = val.split(',').map(|arg| arg.split_once('=').ok_or_else(|| FindTargetError::ArgumentSyntax(arg.to_owned()))).collect::<Result<Vec<_>, _>>()?;
                for (objective, score) in scores {
                    let Some((_, scores)) = scoreboard.iter().find(|(name, _)| name.name() == objective) else {
                        return Err(FindTargetError::ObjectiveNotFound(objective.to_owned()));
                    };
                    if let Some((min, max)) = score.split_once("..") {
                        let (min, max) = (
                            (!min.is_empty()).then_some(min.parse::<i32>().map_err(|_| FindTargetError::InvalidNumber(score.to_owned()))?),
                            (!max.is_empty()).then_some(max.parse::<i32>().map_err(|_| FindTargetError::InvalidNumber(score.to_owned()))?),
                        );
                        targets.retain(|e| players.get(*e).is_ok_and(|(_, _, _, name, ..)| scores.get(&name.0).is_some_and(|score| (min..max).contains(&Some(*score)))));
                    } else {
                        let score = score.parse::<i32>().map_err(|_| FindTargetError::InvalidNumber(score.to_owned()))?;
                        targets.retain(|e| players.get(*e).is_ok_and(|(_, _, _, name, ..)| scores.get(&name.0).is_some_and(|s| *s == score)));
                    }
                }
            } else if arg == "sort" {
                let Ok((_, _, Position(executor_pos), ..)) = players.get(event.executor) else {
                    return Err(FindTargetError::PositionNotFound);
                };
                match val {
                    "nearest" => {
                        targets.sort_by(|target_a, target_b| {
                            let Ok((dist_a, dist_b)) = 
                                living_entities.get(*target_a).map(|(.., p, _)| p.distance(*executor_pos)).and_then(|dist_a|
                                    Ok((dist_a, living_entities.get(*target_b).map(|(.., p, _)| p.distance(*executor_pos))?))) else {
                                return Ordering::Equal;
                            };
                            dist_a.partial_cmp(&dist_b).unwrap_or(Ordering::Equal)
                        });
                    }
                    "furthest" => {
                        targets.sort_by(|target_a, target_b| {
                            let Ok((dist_a, dist_b)) = 
                                living_entities.get(*target_a).map(|(.., p, _)| p.distance(*executor_pos)).and_then(|dist_a|
                                    Ok((dist_a, living_entities.get(*target_b).map(|(.., p, _)| p.distance(*executor_pos))?))) else {
                                return Ordering::Equal;
                            };
                            dist_b.partial_cmp(&dist_a).unwrap_or(Ordering::Equal)
                        });
                    }
                    "random" => {
                        targets.shuffle(&mut thread_rng());
                    }
                    "arbitrary" => {}
                    invalid => {
                        return Err(FindTargetError::InvalidSort(invalid.into()));
                    }
                }
            } else if arg == "level" {
                if let Some((min, max)) = val.split_once("..") {
                    let (min, max) = (
                        (!min.is_empty()).then_some(min.parse::<i32>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?),
                        (!max.is_empty()).then_some(max.parse::<i32>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?),
                    );
                    targets.retain(|e| players.get(*e).is_ok_and(|(.., l, _)| (min..max).contains(&Some(l.level))));
                } else {
                    let val = val.parse::<i32>().map_err(|_| FindTargetError::InvalidNumber(val.to_owned()))?;
                    targets.retain(|e| players.get(*e).is_ok_and(|(.., l, _)| l.level == val));
                }
            } else if arg == "gamemode" {
                let mode = match val.trim_start_matches('!') {
                    "survival" => GameMode::Survival,
                    "creative" => GameMode::Creative,
                    "adventure" => GameMode::Adventure,
                    "spectator" => GameMode::Spectator,
                    invalid => {
                        return Err(FindTargetError::InvalidGamemode(invalid.into()));
                    }
                };
                if let Some('!') = val.chars().next() {
                    targets.retain(|e| players.get(*e).is_ok_and(|(.., g)| *g != mode));
                } else {
                    targets.retain(|e| players.get(*e).is_ok_and(|(.., g)| *g == mode));
                }
            } else if arg == "type" {
                let kind = match val.trim_start_matches("minecraft:") {
                    "allay" => EntityKind::ALLAY,
                    "area_effect_cloud" => EntityKind::AREA_EFFECT_CLOUD,
                    "armor_stand" => EntityKind::ARMOR_STAND,
                    "arrow" => EntityKind::ARROW,
                    "axolotl" => EntityKind::AXOLOTL,
                    "bat" => EntityKind::BAT,
                    "bee" => EntityKind::BEE,
                    "blaze" => EntityKind::BLAZE,
                    "block_display" => EntityKind::BLOCK_DISPLAY,
                    "boat" => EntityKind::BOAT,
                    "camel" => EntityKind::CAMEL,
                    "cat" => EntityKind::CAT,
                    "cave_spider" => EntityKind::CAVE_SPIDER,
                    "chest_boat" => EntityKind::CHEST_BOAT,
                    "chest_minecart" => EntityKind::CHEST_MINECART,
                    "chicken" => EntityKind::CHICKEN,
                    "cod" => EntityKind::COD,
                    "command_block_minecart" => EntityKind::COMMAND_BLOCK_MINECART,
                    "cow" => EntityKind::COW,
                    "creeper" => EntityKind::CREEPER,
                    "dolphin" => EntityKind::DOLPHIN,
                    "donkey" => EntityKind::DONKEY,
                    "dragon_fireball" => EntityKind::DRAGON_FIREBALL,
                    "drowned" => EntityKind::DROWNED,
                    "egg" => EntityKind::EGG,
                    "elder_guardian" => EntityKind::ELDER_GUARDIAN,
                    "end_crystal" => EntityKind::END_CRYSTAL,
                    "ender_dragon" => EntityKind::ENDER_DRAGON,
                    "ender_pearl" => EntityKind::ENDER_PEARL,
                    "enderman" => EntityKind::ENDERMAN,
                    "endermite" => EntityKind::ENDERMITE,
                    "evoker" => EntityKind::EVOKER,
                    "evoker_fangs" => EntityKind::EVOKER_FANGS,
                    "experience_bottle" => EntityKind::EXPERIENCE_BOTTLE,
                    "experience_orb" => EntityKind::EXPERIENCE_ORB,
                    "eye_of_ender" => EntityKind::EYE_OF_ENDER,
                    "falling_block" => EntityKind::FALLING_BLOCK,
                    "fireball" => EntityKind::FIREBALL,
                    "firework_rocket" => EntityKind::FIREWORK_ROCKET,
                    "fishing_bobber" => EntityKind::FISHING_BOBBER,
                    "fox" => EntityKind::FOX,
                    "frog" => EntityKind::FROG,
                    "furnace_minecart" => EntityKind::FURNACE_MINECART,
                    "ghast" => EntityKind::GHAST,
                    "giant" => EntityKind::GIANT,
                    "glow_item_frame" => EntityKind::GLOW_ITEM_FRAME,
                    "glow_squid" => EntityKind::GLOW_SQUID,
                    "goat" => EntityKind::GOAT,
                    "guardian" => EntityKind::GUARDIAN,
                    "hoglin" => EntityKind::HOGLIN,
                    "hopper_minecart" => EntityKind::HOPPER_MINECART,
                    "horse" => EntityKind::HORSE,
                    "husk" => EntityKind::HUSK,
                    "illusioner" => EntityKind::ILLUSIONER,
                    "interaction" => EntityKind::INTERACTION,
                    "iron_golem" => EntityKind::IRON_GOLEM,
                    "item_display" => EntityKind::ITEM_DISPLAY,
                    "item" => EntityKind::ITEM,
                    "item_frame" => EntityKind::ITEM_FRAME,
                    "leash_knot" => EntityKind::LEASH_KNOT,
                    "lightning" => EntityKind::LIGHTNING,
                    "llama" => EntityKind::LLAMA,
                    "llama_spit" => EntityKind::LLAMA_SPIT,
                    "magma_cube" => EntityKind::MAGMA_CUBE,
                    "marker" => EntityKind::MARKER,
                    "minecart" => EntityKind::MINECART,
                    "mooshroom" => EntityKind::MOOSHROOM,
                    "mule" => EntityKind::MULE,
                    "ocelot" => EntityKind::OCELOT,
                    "painting" => EntityKind::PAINTING,
                    "panda" => EntityKind::PANDA,
                    "parrot" => EntityKind::PARROT,
                    "phantom" => EntityKind::PHANTOM,
                    "pig" => EntityKind::PIG,
                    "piglin_brute" => EntityKind::PIGLIN_BRUTE,
                    "piglin" => EntityKind::PIGLIN,
                    "pillager" => EntityKind::PILLAGER,
                    "player" => EntityKind::PLAYER,
                    "polar_bear" => EntityKind::POLAR_BEAR,
                    "potion" => EntityKind::POTION,
                    "pufferfish" => EntityKind::PUFFERFISH,
                    "rabbit" => EntityKind::RABBIT,
                    "ravager" => EntityKind::RAVAGER,
                    "salmon" => EntityKind::SALMON,
                    "sheep" => EntityKind::SHEEP,
                    "shulker_bullet" => EntityKind::SHULKER_BULLET,
                    "shulker" => EntityKind::SHULKER,
                    "silverfish" => EntityKind::SILVERFISH,
                    "skeleton" => EntityKind::SKELETON,
                    "skeleton_horse" => EntityKind::SKELETON_HORSE,
                    "slime" => EntityKind::SLIME,
                    "small_fireball" => EntityKind::SMALL_FIREBALL,
                    "sniffer" => EntityKind::SNIFFER,
                    "snow_golem" => EntityKind::SNOW_GOLEM,
                    "snowball" => EntityKind::SNOWBALL,
                    "spawner_minecart" => EntityKind::SPAWNER_MINECART,
                    "spectral_arrow" => EntityKind::SPECTRAL_ARROW,
                    "spider" => EntityKind::SPIDER,
                    "squid" => EntityKind::SQUID,
                    "stray" => EntityKind::STRAY,
                    "strider" => EntityKind::STRIDER,
                    "tadpole" => EntityKind::TADPOLE,
                    "text_display" => EntityKind::TEXT_DISPLAY,
                    "tnt" => EntityKind::TNT,
                    "tnt_minecart" => EntityKind::TNT_MINECART,
                    "trader_llama" => EntityKind::TRADER_LLAMA,
                    "trident" => EntityKind::TRIDENT,
                    "tropical_fish" => EntityKind::TROPICAL_FISH,
                    "turtle" => EntityKind::TURTLE,
                    "vex" => EntityKind::VEX,
                    "villager" => EntityKind::VILLAGER,
                    "vindicator" => EntityKind::VINDICATOR,
                    "wandering_trader" => EntityKind::WANDERING_TRADER,
                    "warden" => EntityKind::WARDEN,
                    "witch" => EntityKind::WITCH,
                    "wither" => EntityKind::WITHER,
                    "wither_skeleton" => EntityKind::WITHER_SKELETON,
                    "wither_skull" => EntityKind::WITHER_SKULL,
                    "wolf" => EntityKind::WOLF,
                    "zoglin" => EntityKind::ZOGLIN,
                    "zombie" => EntityKind::ZOMBIE,
                    "zombie_horse" => EntityKind::ZOMBIE_HORSE,
                    "zombie_villager" => EntityKind::ZOMBIE_VILLAGER,
                    "zombified_piglin" => EntityKind::ZOMBIFIED_PIGLIN,
                    invalid => {
                        return Err(FindTargetError::InvalidType(invalid.into()));
                    }
                };
                targets.retain(|e| living_entities.get(*e).is_ok_and(|(.., k)| *k == kind));
            } else {
                return Err(FindTargetError::ArgumentNotFound(arg.to_owned()));
            }
        }
        if let (Some(x_pos), Some(y_pos), Some(z_pos), Some(dx), Some(dy), Some(dz)) = (x_pos, y_pos, z_pos, dx, dy, dz) {
            let min = DVec3::new(x_pos, y_pos, z_pos);
            let diff = DVec3::new(dx, dy, dz);
            let volume = Aabb::new(min, min + diff);
            targets.retain(|e| living_entities.get(*e).is_ok_and(|(.., p, _)| volume.contains_point(p.0)));
        }
        Ok(targets)
    } else if let EntitySelectors::NearestPlayer = simple {
        targets.and_then(|mut t| { t.truncate(1); if t.is_empty() { Err(FindTargetError::NearestNotFound) } else { Ok(t) }})
    } else if let EntitySelectors::RandomPlayer = simple {
        targets.into_iter().choose(&mut thread_rng()).ok_or(FindTargetError::RandomNotFound)
    } else {
        targets
    }
}

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::{self, BufReader, ErrorKind, Read, Write};

use flate2::bufread::{GzDecoder, GzEncoder};
use flate2::Compression;
use thiserror::Error;
use valence::block::{PropName, PropValue};
use valence::entity::player::{Food, PlayerEntityBundle, Saturation, Score};
use valence::entity::{Position, Look};
use valence::inventory::{Inventory, InventoryKind};
use valence::layer::chunk::{Chunk, UnloadedChunk};
use valence::math::DVec3;
use valence::nbt::{compound, Compound, List, Value};
use valence::protocol::BlockKind;
use valence::registry::biome::BiomeId;
use valence::registry::BiomeRegistry;
use valence::uuid::Uuid;
use valence::{BlockState, ChunkPos, GameMode, Ident, ItemKind, ItemStack, UniqueId};

use valence::anvil::{RegionError, RegionFolder};

use crate::players::{PlayerData, Xp};

#[derive(Debug)]
pub struct DimensionFolder {
    root: PathBuf,
    region: RegionFolder,
    /// Mapping of biome names to their biome ID.
    biome_to_id: BTreeMap<Ident<String>, BiomeId>,
}

impl DimensionFolder {
    pub fn new<R: Into<PathBuf>>(dimension_root: R, biomes: &BiomeRegistry) -> Self {
        let dimension_root = dimension_root.into();
        let mut region_root = dimension_root.clone();
        region_root.push("region");

        Self {
            root: dimension_root,
            region: RegionFolder::new(region_root),
            biome_to_id: biomes
                .iter()
                .map(|(id, name, _)| (name.to_string_ident(), id))
                .collect(),
        }
    }

    /// Gets the parsed chunk at the given chunk position.
    ///
    /// Returns `Ok(Some(chunk))` if the chunk exists and no errors occurred
    /// loading it. Returns `Ok(None)` if the chunk does not exist and no
    /// errors occurred attempting to load it. Returns `Err(_)` if an error
    /// occurred attempting to load the chunk.
    pub fn get_chunk(&mut self, pos: ChunkPos) -> Result<Option<ParsedChunk>, ParseChunkError> {
        let Some(raw_chunk) = self.region.get_chunk(pos.x, pos.z)? else {
            return Ok(None);
        };
        let parsed = parse_chunk(raw_chunk.data, &self.biome_to_id)?;
        Ok(Some(ParsedChunk {
            chunk: parsed,
            timestamp: raw_chunk.timestamp,
        }))
    }

    pub fn set_chunk<C: Chunk>(&mut self, pos: ChunkPos, chunk: &C) {
        let encoded_chunk = encode_chunk(pos, chunk);
        let _ = self.region.set_chunk(pos.x, pos.z, &encoded_chunk);
    }

    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    pub fn get_player(&mut self, uuid: UniqueId) -> Result<Option<PlayerData>, ParsePlayerError> {
        let world_root = self.root();
        let player_dat_path = world_root
            .join("playerdata")
            .join(format!("{}.dat", uuid.0));
        let player_dat = match File::open(&player_dat_path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(err) => {
                return Err(ParsePlayerError::File(player_dat_path.to_string_lossy().into_owned(), err));
            }
        };
        let player_dat_buffered = BufReader::new(player_dat);

        let mut data = Vec::new();
        let mut dec = GzDecoder::new(player_dat_buffered);
        if let Err(err) = dec.read_to_end(&mut data) {
            return Err(ParsePlayerError::GZip(uuid.0, err));
        }
        let mut data_slice = data.as_slice();

        let nbt = match valence::nbt::from_binary::<String>(&mut data_slice) {
            Ok((nbt, _)) => nbt,
            Err(err) => {
                return Err(ParsePlayerError::Nbt(uuid.0, err));
            }
        };

        if !data_slice.is_empty() {
            return Err(ParsePlayerError::Trailing(uuid.0));
        }

        parse_player(nbt, uuid).map(Some)
    }

    pub fn save_player(&mut self, data: PlayerData) {
        let world_root = self.root();
        let player_dat_path = world_root
            .join("playerdata")
            .join(format!("{}.dat", data.entity.uuid.0));
        let Ok(player_dat) = File::open(&player_dat_path) else {
            tracing::warn!("Failed to open data for player {}", data.entity.uuid.0);
            return;
        };
        let player_dat_buffered = BufReader::new(player_dat);

        let mut current_data = Vec::new();
        let mut dec = GzDecoder::new(player_dat_buffered);
        if let Err(err) = dec.read_to_end(&mut current_data) {
            tracing::warn!("Failed to read existing player data for player {}: {err}", data.entity.uuid.0);
            return;
        }
        let mut current_data_slice = current_data.as_slice();
        let (current, root) = match valence::nbt::from_binary(&mut current_data_slice) {
            Ok(ok) => ok,
            Err(err) => {
                tracing::info!("Failed to parse existing player data for player {}: {err}", data.entity.uuid.0);
                return;
            }
        };
        if !current_data_slice.is_empty() {
            tracing::warn!("Trailing data in existing file for player {}", data.entity.uuid.0);
            return;
        }
        let uuid = data.entity.uuid.0;
        let nbt = encode_player(data, current);
        let mut data = Vec::new();
        if let Err(err) = valence::nbt::to_binary(&nbt, &mut data, &root) {
            tracing::warn!("Failed to encode NBT data for player {}: {err}", uuid);
            return;
        }
        let data_buffered = BufReader::new(data.as_slice());
        let mut enc = GzEncoder::new(data_buffered, Compression::fast());
        let mut buf = Vec::new();
        if let Err(err) = enc.read_to_end(&mut buf) {
            tracing::warn!("Failed to write GZip data for player {}: {err}", uuid);
            return;
        }

        let Ok(mut player_dat) = File::create(&player_dat_path) else {
            tracing::warn!("Failed to open data for player {}", uuid);
            return;
        };
        if let Err(err) = player_dat.write_all(&buf) {
            tracing::warn!("Failed to write data for player {}: {err}", uuid);
            return;
        }
    }
}

fn encode_player(data: PlayerData, mut current: Compound) -> Compound {
    let saved_abilities = compound! {
        "flying" => data.flying as i8,
    };
    let mut abilities = if let Some(Value::Compound(c)) = current.remove("abilities") {
        c
    } else {
        Compound::new()
    };
    abilities.extend(saved_abilities);
    let to_save = compound! {
        "Pos" => List::Double([
            data.entity.position.0.x,
            data.entity.position.0.y,
            data.entity.position.0.z,
        ].to_vec()),
        "Rotation" => List::Float([
            data.entity.look.yaw,
            data.entity.look.pitch,
        ].to_vec()),
        "abilities" => abilities,
        "Dimension" => data.dimension.to_string(),
        "foodLevel" => data.entity.player_food.0,
        "foodSaturationLevel" => data.entity.player_saturation.0,
        "Inventory" => List::Compound((0..data.inventory.slot_count()).map(|id| {
            let ItemStack { item, count, nbt } = data.inventory.slot(id);
            let mut c = compound! {
                "Slot" => id as i8,
                "id" => item.to_str(),
                "count" => *count as i32,
            };
            if let Some(nbt) = nbt {
                c.insert("components", Value::Compound(nbt.clone()));
            }
            c
        }).collect()),
        "playerGameType" => data.game_mode as i32,
        "Score" => data.entity.player_score.0,
        "SelectedItemSlot" => data.held_item as i32,
        "XpLevel" => data.xp.level,
        "XpP" => data.xp.bar,
        "XpTotal" => data.entity.player_score.0,
    };
    current.extend(to_save);
    current
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParsePlayerError {
    #[error("Error opening file `{0}`: {1}")]
    File(String, io::Error),
    #[error("Invalid GZip data found while loading player {0}: {1}")]
    GZip(Uuid, io::Error),
    #[error("Invalid NBT data found while loading player {0}: {1}")]
    Nbt(Uuid, valence::nbt::Error),
    #[error("Trailing data found after reading NBT of player {0}")]
    Trailing(Uuid),
    #[error("No tag with name `{0}` found in data for player {1}")]
    TagNotFound(String, Uuid),
    #[error("Invalid data found in data for player {0}: {1}")]
    Invalid(Uuid, String),
}

fn parse_player(mut nbt: Compound, uuid: UniqueId) -> Result<PlayerData, ParsePlayerError> {
    let mut inventory = Inventory::new(InventoryKind::Player);
    let Some(Value::List(slots)) = nbt.remove("Inventory") else {
        return Err(ParsePlayerError::TagNotFound("Inventory".into(), uuid.0));
    };
    if let List::Compound(slots) = slots {
        for mut slot in slots {
            let Some(Value::Byte(n)) = slot.remove("Slot") else {
                return Err(ParsePlayerError::TagNotFound("Slots".into(), uuid.0));
            };
            let Some(Value::Int(count)) = slot.remove("count") else {
                return Err(ParsePlayerError::TagNotFound("count".into(), uuid.0));
            };
            let Some(Value::String(id)) = slot.remove("id") else {
                return Err(ParsePlayerError::TagNotFound("id".into(), uuid.0));
            };
            let Some(item) = ItemKind::from_str(id.trim_start_matches("minecraft:"))
            else {
                return Err(ParsePlayerError::Invalid(uuid.0, format!("{id} is not a valid item")));
            };
            let nbt = slot.remove("components").and_then(|c| {
                if let Value::Compound(c) = c {
                    Some(c)
                } else {
                    None
                }
            });
            inventory.set_slot(
                n as u16,
                ItemStack {
                    count: count as i8,
                    item,
                    nbt,
                },
            );
        }
    }

    let game_mode = match nbt.get("playerGameType") {
        Some(Value::Int(0)) => GameMode::Survival,
        Some(Value::Int(1)) => GameMode::Creative,
        Some(Value::Int(2)) => GameMode::Adventure,
        Some(Value::Int(3)) => GameMode::Spectator,
        _ => {
            return Err(ParsePlayerError::TagNotFound("playerGameType".into(), uuid.0));
        }
    };

    let Some(Value::Int(held)) = nbt.remove("SelectedItemSlot") else {
        return Err(ParsePlayerError::TagNotFound("SelectedItemSlot".into(), uuid.0));
    };

    let Some(Value::Int(food)) = nbt.remove("foodLevel") else {
        return Err(ParsePlayerError::TagNotFound("foodLevel".into(), uuid.0));
    };
    let food = Food(food);

    let Some(Value::Float(saturation)) = nbt.remove("foodSaturationLevel") else {
        return Err(ParsePlayerError::TagNotFound("foodSaturationLevel".into(), uuid.0));
    };
    let saturation = Saturation(saturation);

    let Some(Value::List(List::Double(pos))) = nbt.remove("Pos") else {
        return Err(ParsePlayerError::TagNotFound("Pos".into(), uuid.0));
    };
    if pos.len() < 3 {
        return Err(ParsePlayerError::Invalid(uuid.0, "invalid position".into()));
    }

    let position = Position(DVec3::new(pos[0], pos[1], pos[2]));

    let Some(Value::Compound(mut abilities)) = nbt.remove("abilities") else {
        return Err(ParsePlayerError::TagNotFound("abilities".into(), uuid.0));
    };
    let Some(Value::Byte(flying)) = abilities.remove("flying") else {
        return Err(ParsePlayerError::TagNotFound("flying".into(), uuid.0));
    };
    let flying = flying != 0;

    let Some(Value::Int(level)) = nbt.remove("XpLevel") else {
        return Err(ParsePlayerError::TagNotFound("XpLevel".into(), uuid.0));
    };
    let Some(Value::Float(bar)) = nbt.remove("XpP") else {
        return Err(ParsePlayerError::TagNotFound("XpP".into(), uuid.0));
    };
    let Some(Value::Int(score)) = nbt.remove("XpTotal") else {
        return Err(ParsePlayerError::TagNotFound("XpTotal".into(), uuid.0));
    };
    let xp = Xp { level, bar };

    let Some(Value::List(List::Float(look))) = nbt.remove("Rotation") else {
        return Err(ParsePlayerError::TagNotFound("Rotation".into(), uuid.0));
    };
    if look.len() < 2 {
        return Err(ParsePlayerError::Invalid(uuid.0, "invalid rotation".into()));
    }
    let look = Look { yaw: look[0], pitch: look[1] };

    let Some(Value::String(dimension)) = nbt.remove("Dimension") else {
        return Err(ParsePlayerError::TagNotFound("Dimension".into(), uuid.0));
    };
    let dimension = match Ident::new(dimension) {
        Ok(d) => d.to_string_ident(),
        Err(err) => {
            return Err(ParsePlayerError::Invalid(uuid.0, format!("invalid dimension ID: {err}")));
        }
    };

    Ok(PlayerData {
        inventory,
        game_mode,
        held_item: held as u8,
        flying,
        xp,
        dimension,
        entity: PlayerEntityBundle {
            player_food: food,
            player_saturation: saturation,
            position,
            player_score: Score(score),
            look,
            ..Default::default()
        },
    })
}

/// A chunk parsed to show block information, biome information etc.
#[derive(Debug)]
pub struct ParsedChunk {
    pub chunk: UnloadedChunk,
    pub timestamp: u32,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseChunkError {
    #[error("region error: {0}")]
    Region(#[from] RegionError),
    #[error("missing chunk sections")]
    MissingSections,
    #[error("missing chunk section Y")]
    MissingSectionY,
    #[error("section Y is out of bounds")]
    SectionYOutOfBounds,
    #[error("missing block states")]
    MissingBlockStates,
    #[error("missing block palette")]
    MissingBlockPalette,
    #[error("invalid block palette length")]
    BadBlockPaletteLen,
    #[error("missing block name in palette")]
    MissingBlockName,
    #[error("unknown block name of \"{0}\"")]
    UnknownBlockName(String),
    #[error("unknown property name of \"{0}\"")]
    UnknownPropName(String),
    #[error("property value of block is not a string")]
    BadPropValueType,
    #[error("unknown property value of \"{0}\"")]
    UnknownPropValue(String),
    #[error("missing packed block state data in section")]
    MissingBlockStateData,
    #[error("unexpected number of longs in block state data")]
    BadBlockLongCount,
    #[error("invalid block palette index")]
    BadBlockPaletteIndex,
    #[error("missing biomes")]
    MissingBiomes,
    #[error("missing biome palette")]
    MissingBiomePalette,
    #[error("invalid biome palette length")]
    BadBiomePaletteLen,
    #[error("biome name is not a valid resource identifier")]
    BadBiomeName,
    #[error("missing packed biome data in section")]
    MissingBiomeData,
    #[error("unexpected number of longs in biome data")]
    BadBiomeLongCount,
    #[error("invalid biome palette index")]
    BadBiomePaletteIndex,
    #[error("missing block entities")]
    MissingBlockEntities,
    #[error("missing block entity ident")]
    MissingBlockEntityIdent,
    #[error("invalid block entity ident of \"{0}\"")]
    InvalidBlockEntityName(String),
    #[error("invalid block entity position")]
    InvalidBlockEntityPosition,
}

fn parse_chunk(
    mut nbt: Compound,
    biome_map: &BTreeMap<Ident<String>, BiomeId>, // TODO: replace with biome registry arg.
) -> Result<UnloadedChunk, ParseChunkError> {
    let Some(Value::List(List::Compound(sections))) = nbt.remove("sections") else {
        return Err(ParseChunkError::MissingSections);
    };

    if sections.is_empty() {
        return Ok(UnloadedChunk::new());
    }

    let mut chunk =
        UnloadedChunk::with_height((sections.len() * 16).try_into().unwrap_or(u32::MAX));

    let min_sect_y = i32::from(
        sections
            .iter()
            .filter_map(|sect| {
                if let Some(Value::Byte(sect_y)) = sect.get("Y") {
                    Some(*sect_y)
                } else {
                    None
                }
            })
            .min()
            .unwrap(),
    );

    let mut converted_block_palette = vec![];
    let mut converted_biome_palette = vec![];

    for mut section in sections {
        let Some(Value::Byte(sect_y)) = section.remove("Y") else {
            return Err(ParseChunkError::MissingSectionY);
        };

        let sect_y = (i32::from(sect_y) - min_sect_y) as u32;

        if sect_y >= chunk.height() / 16 {
            return Err(ParseChunkError::SectionYOutOfBounds);
        }

        let Some(Value::Compound(mut block_states)) = section.remove("block_states") else {
            return Err(ParseChunkError::MissingBlockStates);
        };

        let Some(Value::List(List::Compound(palette))) = block_states.remove("palette") else {
            return Err(ParseChunkError::MissingBlockPalette);
        };

        if !(1..BLOCKS_PER_SECTION).contains(&palette.len()) {
            return Err(ParseChunkError::BadBlockPaletteLen);
        }

        converted_block_palette.clear();

        for mut block in palette {
            let Some(Value::String(name)) = block.remove("Name") else {
                return Err(ParseChunkError::MissingBlockName);
            };

            let Some(block_kind) = BlockKind::from_str(ident_path(&name)) else {
                return Err(ParseChunkError::UnknownBlockName(name));
            };

            let mut state = block_kind.to_state();

            if let Some(Value::Compound(properties)) = block.remove("Properties") {
                for (key, value) in properties {
                    let Value::String(value) = value else {
                        return Err(ParseChunkError::BadPropValueType);
                    };

                    let Some(prop_name) = PropName::from_str(&key) else {
                        return Err(ParseChunkError::UnknownPropName(key));
                    };

                    let Some(prop_value) = PropValue::from_str(&value) else {
                        return Err(ParseChunkError::UnknownPropValue(value));
                    };

                    state = state.set(prop_name, prop_value);
                }
            }

            converted_block_palette.push(state);
        }

        if converted_block_palette.len() == 1 {
            chunk.fill_block_state_section(sect_y, converted_block_palette[0]);
        } else {
            debug_assert!(converted_block_palette.len() > 1);

            let Some(Value::LongArray(data)) = block_states.remove("data") else {
                return Err(ParseChunkError::MissingBlockStateData);
            };

            let bits_per_idx = bit_width(converted_block_palette.len() - 1).max(4);
            let idxs_per_long = 64 / bits_per_idx;
            let long_count = BLOCKS_PER_SECTION.div_ceil(idxs_per_long);
            let mask = 2_u64.pow(bits_per_idx as u32) - 1;

            if long_count != data.len() {
                return Err(ParseChunkError::BadBlockLongCount);
            };

            let mut i: u32 = 0;
            for long in data {
                let u64 = long as u64;

                for j in 0..idxs_per_long {
                    if i >= BLOCKS_PER_SECTION as u32 {
                        break;
                    }

                    let idx = (u64 >> (bits_per_idx * j)) & mask;

                    let Some(block) = converted_block_palette.get(idx as usize).copied() else {
                        return Err(ParseChunkError::BadBlockPaletteIndex);
                    };

                    let x = i % 16;
                    let z = i / 16 % 16;
                    let y = i / (16 * 16);

                    chunk.set_block_state(x, sect_y * 16 + y, z, block);

                    i += 1;
                }
            }
        }

        let Some(Value::Compound(biomes)) = section.get("biomes") else {
            return Err(ParseChunkError::MissingBiomes);
        };

        let Some(Value::List(List::String(palette))) = biomes.get("palette") else {
            return Err(ParseChunkError::MissingBiomePalette);
        };

        if !(1..BIOMES_PER_SECTION).contains(&palette.len()) {
            return Err(ParseChunkError::BadBiomePaletteLen);
        }

        converted_biome_palette.clear();

        for biome_name in palette {
            let Ok(ident) = Ident::<Cow<str>>::new(biome_name) else {
                return Err(ParseChunkError::BadBiomeName);
            };

            converted_biome_palette
                .push(biome_map.get(ident.as_str()).copied().unwrap_or_default());
        }

        if converted_biome_palette.len() == 1 {
            chunk.fill_biome_section(sect_y, converted_biome_palette[0]);
        } else {
            debug_assert!(converted_biome_palette.len() > 1);

            let Some(Value::LongArray(data)) = biomes.get("data") else {
                return Err(ParseChunkError::MissingBiomeData);
            };

            let bits_per_idx = bit_width(converted_biome_palette.len() - 1);
            let idxs_per_long = 64 / bits_per_idx;
            let long_count = BIOMES_PER_SECTION.div_ceil(idxs_per_long);
            let mask = 2_u64.pow(bits_per_idx as u32) - 1;

            if long_count != data.len() {
                return Err(ParseChunkError::BadBiomeLongCount);
            };

            let mut i: u32 = 0;
            for &long in data {
                let u64 = long as u64;

                for j in 0..idxs_per_long {
                    if i >= BIOMES_PER_SECTION as u32 {
                        break;
                    }

                    let idx = (u64 >> (bits_per_idx * j)) & mask;

                    let Some(biome) = converted_biome_palette.get(idx as usize).copied() else {
                        return Err(ParseChunkError::BadBiomePaletteIndex);
                    };

                    let x = i % 4;
                    let z = i / 4 % 4;
                    let y = i / (4 * 4);

                    chunk.set_biome(x, sect_y * 4 + y, z, biome);

                    i += 1;
                }
            }
        }
    }

    let Some(Value::List(block_entities)) = nbt.remove("block_entities") else {
        return Err(ParseChunkError::MissingBlockEntities);
    };

    if let List::Compound(block_entities) = block_entities {
        for mut comp in block_entities {
            let Some(Value::String(ident)) = comp.remove("id") else {
                return Err(ParseChunkError::MissingBlockEntityIdent);
            };

            if let Err(e) = Ident::new(ident) {
                return Err(ParseChunkError::InvalidBlockEntityName(e.0));
            }

            let Some(Value::Int(x)) = comp.remove("x") else {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            };

            let x = x.rem_euclid(16) as u32;

            let Some(Value::Int(y)) = comp.remove("y") else {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            };

            let Ok(y) = u32::try_from(y.wrapping_sub(min_sect_y * 16)) else {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            };

            if y >= chunk.height() {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            }

            let Some(Value::Int(z)) = comp.remove("z") else {
                return Err(ParseChunkError::InvalidBlockEntityPosition);
            };

            let z = z.rem_euclid(16) as u32;

            comp.remove("keepPacked");

            chunk.set_block_entity(x, y, z, Some(comp));
        }
    }

    Ok(chunk)
}

const BLOCKS_PER_SECTION: usize = 16 * 16 * 16;
const BIOMES_PER_SECTION: usize = 4 * 4 * 4;

/// Gets the path part of a resource identifier.
fn ident_path(ident: &str) -> &str {
    match ident.rsplit_once(':') {
        Some((_, after)) => after,
        None => ident,
    }
}

/// Returns the minimum number of bits needed to represent the integer `n`.
const fn bit_width(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as usize
}

fn encode_chunk<C: Chunk>(pos: ChunkPos, chunk: &C) -> Compound {
    let mut blocks = Vec::new();
    let mut palette = Vec::<BlockState>::new();
    let sections = (0..24)
        .map(|y| {
            let sect_y = y * 16;
            blocks.clear();
            palette.clear();
            for offset_y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let block = chunk.block_state(x, sect_y + offset_y, z);
                        if let Some((idx, _)) =
                            palette.iter().enumerate().find(|(_, &b)| b == block)
                        {
                            blocks.push(idx);
                        } else {
                            blocks.push(palette.len());
                            palette.push(block);
                        }
                    }
                }
            }

            let encoded_palette = palette
                .iter()
                .map(|b| {
                    let mut nbt = Compound::new();
                    for prop in b.to_kind().props() {
                        nbt.insert(prop.to_str(), b.get(*prop).unwrap().to_str());
                    }
                    compound! {
                        "Name" => format!("minecraft:{}", b.to_kind().to_str()),
                        "Properties" => nbt,
                    }
                })
                .collect();

            if palette.len() > 1 {
                let bits_per_idx = bit_width(palette.len() - 1).max(4);
                let idxs_per_long = 64 / bits_per_idx;
                let long_count = BLOCKS_PER_SECTION.div_ceil(idxs_per_long);

                let data = (0..long_count)
                    .map(|i| {
                        let first = i * idxs_per_long;
                        let mut long = 0_u64;
                        for j in 0..idxs_per_long {
                            if first + j >= BLOCKS_PER_SECTION {
                                break;
                            }
                            long |= (blocks[first + j] as u64) << (j * bits_per_idx);
                        }
                        long as i64
                    })
                    .collect::<Vec<_>>();
                compound! {
                    "Y" => y as i8,
                    "block_states" => compound! {
                        "palette" => List::Compound(encoded_palette),
                        "data" => Value::LongArray(data),
                    },
                    "biomes" => compound! {
                        "palette" => List::String(vec!["minecraft:plains".to_owned()]),
                    },
                }
            } else {
                compound! {
                    "Y" => y as i8,
                    "block_states" => compound! {
                        "palette" => List::Compound(encoded_palette),
                    },
                    "biomes" => compound! {
                        "palette" => List::String(vec!["minecraft:plains".to_owned()]),
                    },
                }
            }
        })
        .collect();
    compound! {
        "DataVersion" => 3218,
        "xPos" => pos.x,
        "zPos" => pos.z,
        "yPos" => -4,
        "Status" => "minecraft:full",
        "LastUpdate" => 42,
        "sections" => List::Compound(sections),
        "block_entities" => List::Compound(vec![]),
        "Heightmaps" => compound! {
            "MOTION_BLOCKING" => List::Long(vec![0; 37]),
            "MOTION_BLOCKING_NO_LEAVES" => List::Long(vec![0; 37]),
            "OCEAN_FLOOR" => List::Long(vec![0; 37]),
            "OCEAN_FLOOR_WG" => List::Long(vec![0; 37]),
            "WORLD_SURFACE" => List::Long(vec![0; 37]),
            "WORLD_SURFACE_WG" => List::Long(vec![0; 37]),
        },
        "fluid_ticks" => List::Compound(vec![]),
        "block_ticks" => List::Compound(vec![]),
        "InhabitedTime" => 420_i64,
        "blending_data" => compound! {
            "max_section" => 20,
            "min_section" => -4,
        },
        "structures" => compound! {
            "References" => compound! {},
            "starts" => compound! {},
        },
    }
}

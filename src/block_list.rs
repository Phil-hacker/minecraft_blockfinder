use bevy::{
    log::*,
    utils::{hashbrown::HashSet, HashMap},
};
use bevy_meshem::prelude::Face::{self, *};
use minecraft_assets::{
    api::AssetPack,
    schemas::{
        blockstates::{ModelProperties, Variant},
        models::{BlockFace, ElementFace, Texture},
    },
};

#[derive(Debug)]
struct BlockPath<'a>(&'a str, &'a str);

static BLOCK_LIST: &[BlockPath] = &[
    BlockPath("grass_block", "snowy=false"),
    // BlockPath("stone", ""),
    // BlockPath("dirt", ""),
    // BlockPath("sand", ""),
    // BlockPath("gravel", ""),
    // BlockPath("granite", ""),
    // BlockPath("diorite", ""),
    // BlockPath("andesite", ""),
    // BlockPath("deepslate", ""),
    // BlockPath("calcite", ""),
    // BlockPath("cobblestone", ""),
    // BlockPath("end_stone", ""),
    // BlockPath("netherrack", ""),
    // BlockPath("red_sand", ""),
    // BlockPath("rooted_dirt", ""),
];

#[derive(Debug, bevy::ecs::system::Resource)]
pub struct BlockList {
    pub blocks: HashMap<String, Block>,
}

impl BlockList {
    pub fn get_textures(&self) -> Vec<(String, String)> {
        let textures: HashSet<(String, String)> = self
            .blocks
            .values()
            .flat_map(|v| v.0.iter().map(|v| v.faces.get_textures()))
            .flatten()
            .map(|texture| {
                (
                    texture.clone(),
                    format!("minecraft\\textures\\{}.png", texture),
                )
            })
            .collect();
        Vec::from_iter(textures)
    }

    pub fn new(path: &str) -> Self {
        Self {
            blocks: load_models(path),
        }
    }
}

fn load_models(path: &str) -> HashMap<String, Block> {
    let assets = AssetPack::at_path(path);
    let blocks = load_blocks(&assets);
    blocks
        .into_iter()
        .map(|(id, block)| {
            (
                id,
                Block(match block {
                    Variant::Multiple(variants) => variants
                        .iter()
                        .map(|variant| process_model_properties(variant, &assets))
                        .collect(),
                    Variant::Single(variant) => vec![process_model_properties(&variant, &assets)],
                }),
            )
        })
        .collect()
}

pub fn process_model_properties(properties: &ModelProperties, assets: &AssetPack) -> BlockVariant {
    let models = assets
        .load_block_model_recursive(&properties.model)
        .unwrap();
    let mut elements = Vec::new();
    let mut textures = HashMap::new();
    for model in models {
        if let Some(model) = model.textures {
            textures.extend(model.variables)
        };
        if elements.is_empty() {
            if let Some(e) = model.elements {
                elements = e;
            }
        }
    }
    let element = elements.into_iter().next().unwrap();
    let faces = element.faces.into_iter().collect();
    BlockVariant {
        faces: Faces::from((textures, faces)),
        x: properties.x,
        y: properties.y,
    }
}

#[inline]
pub fn load_blocks(assets: &AssetPack) -> Vec<(String, Variant)> {
    BLOCK_LIST
        .iter()
        .filter_map(|block| {
            if let Ok(blockstates) = assets.load_blockstates(block.0) {
                let variants = blockstates.variants()?;
                let model = variants.get(&(*block.1).to_owned())?;
                Some((block.0.to_string(), model.clone()))
            } else {
                warn!("couldn't load {block:?}");
                None
            }
        })
        .collect()
}

#[derive(Debug)]
pub struct Block(pub Vec<BlockVariant>);

#[derive(Debug)]
pub struct BlockVariant {
    pub faces: Faces,
    pub x: i32,
    pub y: i32,
}

impl BlockVariant {
    pub fn get_textures(&self, texture_map: &HashMap<String, usize>) -> [(Face, [u32; 2]); 6] {
        info!("{},{},{}", self.x, self.y, &self.faces.top.0);

        return [
            (
                Top,
                [*texture_map.get(&self.faces.top.0).unwrap() as u32, 0],
            ),
            (
                Bottom,
                [*texture_map.get(&self.faces.bottom.0).unwrap() as u32, 0],
            ),
            (
                Forward,
                [*texture_map.get(&self.faces.north.0).unwrap() as u32, 0],
            ),
            (
                Back,
                [*texture_map.get(&self.faces.south.0).unwrap() as u32, 0],
            ),
            (
                Right,
                [*texture_map.get(&self.faces.east.0).unwrap() as u32, 0],
            ),
            (
                Left,
                [*texture_map.get(&self.faces.west.0).unwrap() as u32, 0],
            ),
        ];
    }
}

#[derive(Debug)]
pub struct Faces {
    pub top: (String, u8),
    pub bottom: (String, u8),
    pub south: (String, u8),
    pub north: (String, u8),
    pub west: (String, u8),
    pub east: (String, u8),
}

impl From<(HashMap<String, Texture>, HashMap<BlockFace, ElementFace>)> for Faces {
    fn from(value: (HashMap<String, Texture>, HashMap<BlockFace, ElementFace>)) -> Self {
        let (textures, faces) = value;
        Self {
            top: (
                get_texture(&textures, "#top"),
                faces
                    .get(&BlockFace::Up)
                    .map(|v| v.rotation as u8)
                    .unwrap_or_default(),
            ),
            bottom: (
                get_texture(&textures, "#bottom"),
                faces
                    .get(&BlockFace::Down)
                    .map(|v| v.rotation as u8)
                    .unwrap_or_default(),
            ),
            north: (
                get_texture(&textures, "#north"),
                faces
                    .get(&BlockFace::North)
                    .map(|v| v.rotation as u8)
                    .unwrap_or_default(),
            ),
            south: (
                get_texture(&textures, "#south"),
                faces
                    .get(&BlockFace::South)
                    .map(|v| v.rotation as u8)
                    .unwrap_or_default(),
            ),
            west: (
                get_texture(&textures, "#west"),
                faces
                    .get(&BlockFace::West)
                    .map(|v| v.rotation as u8)
                    .unwrap_or_default(),
            ),
            east: (
                get_texture(&textures, "#east"),
                faces
                    .get(&BlockFace::East)
                    .map(|v| v.rotation as u8)
                    .unwrap_or_default(),
            ),
        }
    }
}
fn get_texture(textures: &HashMap<String, Texture>, texture: &str) -> String {
    let side_texture = Texture::from("#side");
    if let Some(value) = textures
        .get(texture.trim_start_matches('#'))
        .or(match texture {
            "#north" | "#west" | "#south" | "#east" => Some(&side_texture),
            _ => None,
        })
    {
        if value.0.starts_with('#') {
            get_texture(textures, &value.0)
        } else {
            value
                .location()
                .unwrap()
                .trim_start_matches("minecraft:")
                .into()
        }
    } else {
        match texture {
            "#top" => get_texture(textures, "#up"),
            "#bottom" => get_texture(textures, "#down"),
            _ => {
                error!(
                    "couldn't get {} in {:?}",
                    texture.trim_start_matches('#'),
                    textures
                );
                panic!();
            }
        }
    }
}

impl Faces {
    pub fn get_textures(&self) -> Vec<String> {
        vec![
            self.top.0.clone(),
            self.bottom.0.clone(),
            self.south.0.clone(),
            self.north.0.clone(),
            self.west.0.clone(),
            self.east.0.clone(),
        ]
    }
}

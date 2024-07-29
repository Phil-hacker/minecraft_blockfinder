use crate::constants::*;
use bevy::render::mesh::{Mesh, MeshVertexAttribute};
use bevy::{asset::LoadState, prelude::*, utils::HashMap};
use bevy_meshem::{prelude::generate_voxel_mesh, VoxelMesh, VoxelRegistry};
use rand::prelude::SliceRandom;
use std::fmt::Debug;

use crate::{block_list::BlockList, AppState};

#[derive(Debug, Resource)]
pub struct MinecraftAssets(HashMap<String, Handle<Image>>);
#[derive(Resource)]
pub struct MinecraftBlockProvider {
    block_material: Handle<StandardMaterial>,
    blocks: Vec<BlockMeta>,
    block_map: HashMap<String, usize>,
    meshes: HashMap<BlockId, Mesh>,
}

#[derive(Clone, Copy, Debug)]
pub struct BlockMeta {
    pub id: usize,
    pub variants: u8,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct BlockId(pub usize, pub u8);

impl Debug for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == AIR {
            f.write_str("Air")
        } else {
            f.debug_tuple("BlockId")
                .field(&self.0)
                .field(&self.1)
                .finish()
        }
    }
}

impl VoxelRegistry for MinecraftBlockProvider {
    type Voxel = BlockId;
    fn get_mesh(&self, voxel: &Self::Voxel) -> VoxelMesh<&Mesh> {
        self.meshes
            .get(voxel)
            .map(VoxelMesh::NormalCube)
            .unwrap_or(VoxelMesh::Null)
    }
    fn is_covering(&self, voxel: &Self::Voxel, _side: bevy_meshem::prelude::Face) -> bool {
        self.meshes.get(voxel).is_some() && *voxel != AIR
    }

    fn get_center(&self) -> [f32; 3] {
        VOXEL_CENTER
    }

    fn get_voxel_dimensions(&self) -> [f32; 3] {
        VOXEL_DIMS
    }

    fn all_attributes(&self) -> Vec<MeshVertexAttribute> {
        vec![Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_UV_0]
    }
}

impl MinecraftBlockProvider {
    pub fn get_random_block(&self) -> &BlockMeta {
        self.blocks.choose(&mut rand::thread_rng()).unwrap()
    }
    pub fn get_block_material(&self) -> Handle<StandardMaterial> {
        self.block_material.clone()
    }
    pub fn get_blocks(&self) -> Vec<&str> {
        self.block_map.keys().map(|v| v.as_str()).collect()
    }
    pub fn get_meta<'a>(&'a self, id: &str) -> &'a BlockMeta {
        self.blocks.get(*self.block_map.get(id).unwrap()).unwrap()
    }
    pub fn get_meta_from_index(&self, index: usize) -> Option<&BlockMeta> {
        self.blocks.get(index)
    }
}

impl MinecraftAssets {
    pub fn from_blocklist(block_list: &BlockList, asset_server: &AssetServer) -> Self {
        Self(
            block_list
                .get_textures()
                .into_iter()
                .map(|(v, mut path)| {
                    info!("{path}");
                    if std::env::var("TEST_ASSETS")
                        .map(|v| v.is_empty())
                        .unwrap_or_default()
                    {
                        path = format!("test_assets/assets/{path}");
                    }
                    let handle = asset_server.load(path);
                    (v, handle)
                })
                .collect(),
        )
    }
    pub fn finished_loading(&self, asset_server: &AssetServer) -> bool {
        self.0.values().all(|handle| {
            asset_server
                .get_load_state(handle.id())
                .map(|loading_state| loading_state == LoadState::Loaded)
                .unwrap_or(false)
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(OnEnter(AppState::Loading), load_assets)
            .add_systems(Update, check_assets.run_if(in_state(AppState::Loading)))
            .add_systems(OnExit(AppState::Loading), generate_block_db);
    }
}

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("start loading");
    #[cfg(not(feature = "test_assets"))]
    let block_list = BlockList::new(".");
    #[cfg(feature = "test_assets")]
    let block_list = BlockList::new("./assets/test_assets");
    let minecraft_assets = MinecraftAssets::from_blocklist(&block_list, &asset_server);
    commands.insert_resource(minecraft_assets);
    commands.insert_resource(block_list);
}

fn check_assets(
    mut state: ResMut<NextState<AppState>>,
    minecraft_assets: Res<MinecraftAssets>,
    asset_server: Res<AssetServer>,
) {
    if minecraft_assets.finished_loading(&asset_server) {
        *state.as_mut() = NextState::Pending(AppState::BuildingInit);
        info!("finished loading")
    }
}

fn generate_block_db(
    mut commands: Commands,
    minecraft_assets: Res<MinecraftAssets>,
    block_list: Res<BlockList>,
    mut textures: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("creating atlas");
    info!("{:?}", minecraft_assets);
    let size = textures
        .get(minecraft_assets.0.values().next().unwrap())
        .unwrap()
        .size();
    let mut atlas_builder = TextureAtlasBuilder::default();
    atlas_builder.initial_size(UVec2::new(size.x * minecraft_assets.0.len() as u32, size.y));
    let mut texture_map = HashMap::default();
    for (id, texture) in minecraft_assets.0.iter() {
        atlas_builder.add_texture(Some(texture.id()), textures.get(texture).unwrap());
        texture_map.insert(id.clone(), texture.clone());
    }
    let (atlas_layout, atlas_texture) = atlas_builder.build().unwrap();
    let texture_map = texture_map
        .into_iter()
        .map(|(k, v)| (k, atlas_layout.get_texture_index(v.id()).unwrap()))
        .collect();
    let mut variant_map = HashMap::default();
    let mut block_map = HashMap::default();
    let mut blocks = Vec::default();
    for (num, (id, block)) in block_list.blocks.iter().enumerate() {
        block_map.insert(id.clone(), num);
        for (variant_num, variant) in block.0.iter().enumerate() {
            let mesh = generate_voxel_mesh(
                VOXEL_DIMS,
                [atlas_layout.len() as u32, 1],
                variant.get_textures(&texture_map),
                VOXEL_CENTER,
                0.0,
                None,
                1.0,
            );
            variant_map.insert(
                BlockId(num, variant_num as u8),
                rotate_mesh(mesh, variant.x, variant.y),
            );
            blocks.push(BlockMeta {
                id: num,
                variants: block.0.len() as u8,
            });
        }
    }
    let texture_handle: Handle<Image> = textures.add(atlas_texture);
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(texture_handle),
        reflectance: 0.0,
        alpha_mode: AlphaMode::Mask(0.3),
        perceptual_roughness: 0.75,
        ..default()
    });

    let minecraft_block_provider = MinecraftBlockProvider {
        block_material: mat,
        block_map,
        blocks,
        meshes: variant_map,
    };
    commands.insert_resource(minecraft_block_provider);
    info!("finished creating atlas");
}

pub fn rotate_mesh(mut mesh: Mesh, _x: i32, y: i32) -> Mesh {
    /*
    if x==0 && y==0 {
        return mesh;
    }
    */
    mesh = flip_sides(mesh);
    if let Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uv)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
    {
        /*
        for _ in 0..(-x / 90).abs() {
            // Top: 0..=3, Bottom: 4..=7, Right: 8..=11, Left: 12..=15, Back: 16..=19, Forward: 20..=23
            for i in 0..3 {
                uv.swap(8 + i, 9 + i); // rotate right
                uv.swap(12 + i, 13 + i); // rotate left
            }
            for (i, j) in [(0, 1), (1, 4), (4, 5)] {
                uv.swap(i * 4, j * 4);
                uv.swap(i * 4 + 1, j * 4 + 1);
                uv.swap(i * 4 + 2, j * 4 + 2);
                uv.swap(i * 4 + 3, j * 4 + 3);
            }
        }
        */
        for _ in 0..2 + (y / 90).abs() {
            // Top: 0..=3, Bottom: 4..=7, Right: 8..=11, Left: 12..=15, Back: 16..=19, Forward: 20..=23
            for i in 0..3 {
                uv.swap(i, i + 1); // rotate top
                uv.swap(4 + i, 5 + i); // rotate bottom
            }
            /*
            for i in 2..5 {
                for j in 0..4 {
                    uv.swap(i * 4 + j, (i + 1) * 4 + j)
                }
            }
             */
        }
    } else {
        unreachable!()
    }

    mesh
}

pub fn flip_sides(mut mesh: Mesh) -> Mesh {
    if let Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uv)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
    {
        for i in 0..6 {
            uv.swap(i * 4, i * 4 + 1);
            uv.swap(i * 4 + 2, i * 4 + 3);
        }
    } else {
        unreachable!()
    }
    mesh
}

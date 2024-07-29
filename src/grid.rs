use bevy::prelude::*;
use bevy_meshem::prelude::*;
use bevy_mod_raycast::prelude::*;

use crate::finder::Rotation;
use crate::game_assets::{BlockId, MinecraftBlockProvider};
use crate::{constants::*, AppState};

#[derive(Resource)]
pub struct Grid {
    pub grid: Box<[BlockId; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]>,
    metadata: MeshMD<BlockId>,
}

#[derive(Component)]
pub struct GridMesh;

impl Grid {
    pub fn init_grid(
        meshes: &mut Assets<Mesh>,
        voxel_registry: &MinecraftBlockProvider,
        material: &Handle<StandardMaterial>,
    ) -> (Self, PbrBundle) {
        let grid = Box::new([AIR; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]);
        let (mesh, metadata) = mesh_grid(
            GRID_SIZE,
            &[],
            grid.as_ref(),
            voxel_registry,
            bevy_meshem::prelude::MeshingAlgorithm::Culling,
            None,
        )
        .unwrap();
        (
            Grid { grid, metadata },
            PbrBundle {
                mesh: meshes.add(mesh),
                material: material.clone(),
                ..Default::default()
            },
        )
    }
    pub fn add_block(
        &mut self,
        voxel_index: usize,
        voxel: BlockId,
        mesh: &mut Mesh,
        voxel_registry: &MinecraftBlockProvider,
    ) {
        let neighbors = {
            let mut neighboring_voxels = [None; 6];
            for (i, item) in neighboring_voxels.iter_mut().enumerate() {
                *item = if let Some(a) = get_neighbor(voxel_index, Face::from(i), GRID_SIZE) {
                    Some(self.grid[a])
                } else {
                    continue;
                }
            }
            neighboring_voxels
        };
        self.metadata.log(
            bevy_meshem::prelude::VoxelChange::Added,
            voxel_index,
            voxel,
            neighbors,
        );
        self.grid[voxel_index] = voxel;
        update_mesh(mesh, &mut self.metadata, voxel_registry);
    }
    pub fn rotate_block(
        &mut self,
        voxel_index: usize,
        mesh: &mut Mesh,
        voxel_registry: &MinecraftBlockProvider,
    ) {
        let mut block = self.grid[voxel_index];
        let block_info = voxel_registry.get_meta_from_index(block.0);
        info!("{block_info:?}, {block:?}");
        block.1 = (block.1 + 1) % block_info.map(|v| v.variants).unwrap_or_default();
        self.remove_block(voxel_index, mesh, voxel_registry);
        self.add_block(voxel_index, block, mesh, voxel_registry);
    }
    pub fn remove_block(
        &mut self,
        voxel_index: usize,
        mesh: &mut Mesh,
        voxel_registry: &MinecraftBlockProvider,
    ) {
        if self.grid[voxel_index] == AIR {
            return;
        }
        let neighbors = {
            let mut neighboring_voxels = [None; 6];
            for (i, item) in neighboring_voxels.iter_mut().enumerate() {
                *item = if let Some(a) = get_neighbor(voxel_index, Face::from(i), GRID_SIZE) {
                    Some(self.grid[a])
                } else {
                    continue;
                }
            }
            neighboring_voxels
        };
        self.metadata.log(
            bevy_meshem::prelude::VoxelChange::Broken,
            voxel_index,
            self.grid[voxel_index],
            neighbors,
        );
        self.grid[voxel_index] = AIR;
        update_mesh(mesh, &mut self.metadata, voxel_registry);
    }
    pub fn reset(&mut self, mesh: &mut Mesh, voxel_registry: &MinecraftBlockProvider) {
        self.grid = Box::new([AIR; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]);
        (*mesh, self.metadata) = mesh_grid(
            GRID_SIZE,
            &[],
            self.grid.as_ref(),
            voxel_registry,
            bevy_meshem::prelude::MeshingAlgorithm::Culling,
            None,
        )
        .unwrap();
    }
    pub fn as_rotations(
        &self,
        block_provider: &MinecraftBlockProvider,
    ) -> Box<[Rotation; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]> {
        let mut array: Box<[Rotation; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]> =
            Box::new([Rotation(0); GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]);
        for (i, element) in self.grid.iter().enumerate() {
            array[i] = Rotation::new(
                element.1,
                block_provider.get_meta_from_index(element.0).map(|v| v.variants).unwrap_or_default(),
            );
        }
        array
    }
    pub fn as_u8(
        &self,
        block_provider: &MinecraftBlockProvider,
    ) -> Box<[u8; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]> {
        let mut array: Box<[u8; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]> =
            Box::new([0; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]);
        for (i, element) in self.grid.iter().enumerate() {
            array[i] = Rotation::new(
                element.1,
                block_provider.get_meta_from_index(element.0).map(|v| v.variants).unwrap_or_default(),
            )
            .0;
        }
        array
    }
}

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(OnEnter(AppState::BuildingInit), init_grid);
    }
}
fn init_grid(
    mut commands: Commands,
    blocks: Res<MinecraftBlockProvider>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut state: ResMut<NextState<AppState>>,
) {
    info!("initializing grid");
    let (grid, mesh) = Grid::init_grid(
        meshes.as_mut(),
        blocks.as_ref(),
        &blocks.get_block_material(),
    );
    commands.insert_resource(grid);
    commands.spawn((mesh, GridMesh, RaycastMesh::<()>::default()));
    *state.as_mut() = NextState::Pending(AppState::Building);
    info!("grid initialized");
}

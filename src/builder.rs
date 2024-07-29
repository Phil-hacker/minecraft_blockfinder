use bevy::{
    prelude::*,
    render::{renderer::RenderDevice, settings::WgpuFeatures},
    window::{PrimaryWindow, WindowResized},
};
use bevy_flycam::FlyCam;
use bevy_meshem::prelude::*;
use bevy_mod_raycast::prelude::*;

use crate::{
    finder::{plugin::FinderJob, util::get_block_rotation},
    game_assets::{BlockId, MinecraftBlockProvider},
    grid::{Grid, GridMesh},
    AppState,
};

use crate::constants::*;

pub struct BuilderPlugin;

#[derive(Component)]
struct BuilderGui;

impl Plugin for BuilderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            OnEnter(AppState::Building),
            (setup_raycast, setup_builder, setup_builder_gui, test_gpu),
        )
        .add_systems(
            Update,
            (handle_raycasts, on_resize, handle_keyboard_inputs).run_if(in_state(AppState::Building)),
        )
        .add_systems(OnExit(AppState::Building), remove_builder_gui);
    }
}

fn test_gpu(mut render_device: ResMut<RenderDevice>) {
    let render_device = render_device.as_mut();
    assert!(render_device
        .features()
        .contains(WgpuFeatures::SHADER_INT64));
}

fn handle_keyboard_inputs(
    inputs: Res<ButtonInput<KeyCode>>,
    voxel_registry: Res<MinecraftBlockProvider>,
    mut grid: ResMut<Grid>,
    mut commands: Commands,
    mut state: ResMut<NextState<AppState>>,
    grid_mesh: Query<&mut Handle<Mesh>, With<GridMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if inputs.just_pressed(KeyCode::Enter) {
        commands.insert_resource(FinderJob(grid.as_ref().as_u8(voxel_registry.as_ref())));
        *state.as_mut() = NextState::Pending(AppState::Searching)
    }
    if inputs.just_pressed(KeyCode::KeyR) {
        let mesh = meshes.get_mut(grid_mesh.single().id()).unwrap();
        let block_meta = voxel_registry.get_meta("grass_block");
        grid.as_mut().reset(mesh, voxel_registry.as_ref());
        let index = one_d_cords([GRID_SIZE.0 / 2, GRID_SIZE.1 / 2, GRID_SIZE.2 / 2,], GRID_SIZE);
        grid.as_mut().add_block(index,
            BlockId(
                block_meta.id,
                0
            ),
            mesh,
            voxel_registry.as_ref(),
        );
    }
}

fn setup_raycast(
    mut commands: Commands,
    window: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(Entity, &Camera, &GlobalTransform), With<FlyCam>>,
) {
    let window = window.single();
    for (entity, camera, transform) in cams.iter() {
        commands
            .entity(entity)
            .insert(RaycastSource::<()>::new_screenspace(
                Vec2 {
                    x: window.width() / 1.66,
                    y: window.height() / 2.0,
                },
                camera,
                transform,
                window,
            ));
    }
    info!("raycast setup complete");
}

fn setup_builder(
    voxel_registry: Res<MinecraftBlockProvider>,
    mut grid: ResMut<Grid>,
    grid_mesh: Query<&mut Handle<Mesh>, With<GridMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    // temporary
) {
    for mesh in grid_mesh.into_iter() {
        info!("{mesh:?}");
        let mesh = meshes.get_mut(mesh.id()).unwrap();
        for position in 0..32 * 32 * 32 {
            //let block_meta = voxel_registry.get_random_block();
            let block_meta = voxel_registry.get_meta("grass_block");
            //let position = rand::random::<usize>() % (GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2);
            let (x, y, z) = three_d_cords(position, GRID_SIZE);
            grid.as_mut().add_block(
                position,
                BlockId(
                    block_meta.id,
                    //get_block_rotation((x) as i64, (y) as i64, (z) as i64)
                    get_block_rotation((x + 9315) as i64, (y + 175) as i64, (z + 6321) as i64)
                        % block_meta.variants,
                ),
                mesh,
                voxel_registry.as_ref(),
            )
        }
    }
    info!("blocks added");
}

fn setup_builder_gui(voxel_registry: Res<MinecraftBlockProvider>, mut commands: Commands) {
    commands
        .spawn((NodeBundle {
            style: Style {
                width: Val::Percent(20.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        }, BuilderGui))
        .with_children(|parent| {
            // left vertical fill (content)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    background_color: Color::srgb(0.15, 0.15, 0.15).into(),
                    ..default()
                })
                .with_children(|parent| {
                    for block in voxel_registry.get_blocks() {
                        voxel_registry.get_meta(block);
                        parent.spawn((
                            TextBundle::from_section(
                                block,
                                TextStyle {
                                    font_size: 30.0,
                                    ..default()
                                },
                            )
                            .with_style(Style {
                                margin: UiRect::all(Val::Px(5.)),
                                ..default()
                            }),
                            Label,
                        ));
                    }
                });
        });
    info!("gui setup complete")
}

fn remove_builder_gui(mut commands: Commands, gui: Query<Entity, With<BuilderGui>>) {
    info!("removing gui");
    for entity in gui.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn handle_raycasts(
    sources: Query<&RaycastSource<()>>,
    mut gizmos: Gizmos,
    buttons: Res<ButtonInput<MouseButton>>,
    voxel_registry: Res<MinecraftBlockProvider>,
    mut grid: ResMut<Grid>,
    grid_mesh: Query<&mut Handle<Mesh>, With<GridMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mesh = grid_mesh.single();
    for source in sources.iter() {
        let ray = source.get_ray().unwrap();
        gizmos.circle(
            ray.origin + Vec3::from(ray.direction),
            ray.direction,
            0.005,
            bevy::color::palettes::css::BLACK,
        );
        if let Some((_, hit)) = source.get_nearest_intersection() {
            let add_block = hit.position() + (hit.normal() / 2.0);
            let remove_block = hit.position() - (hit.normal() / 2.0);
            let add_block = position_to_chunk_position(add_block, GRID_SIZE);
            let remove_block = position_to_chunk_position(remove_block, GRID_SIZE);
            gizmos.cuboid(
                Transform::from_translation(Vec3::from_array(add_block.1.map(|v| v as f32)))
                    .with_scale(Vec3::splat(1.)),
                bevy::color::palettes::css::GREEN,
            );
            gizmos.cuboid(
                Transform::from_translation(Vec3::from_array(remove_block.1.map(|v| v as f32)))
                    .with_scale(Vec3::splat(1.)),
                bevy::color::palettes::css::RED,
            );

            if buttons.just_pressed(MouseButton::Right) {
                let block_meta = voxel_registry.get_meta("grass_block");
                let (chunk, block, valid) = add_block;
                info!("{:?},{:?},{}", chunk, block, valid);
                if chunk == [0, 0] && valid {
                    grid.as_mut().add_block(
                        one_d_cords(block, GRID_SIZE),
                        BlockId(block_meta.id, 0),
                        meshes.get_mut(mesh).unwrap(),
                        voxel_registry.as_ref(),
                    );
                }
            }
            if buttons.just_pressed(MouseButton::Middle) {
                let (chunk, block, valid) = remove_block;
                if chunk == [0, 0] && valid {
                    grid.as_mut().rotate_block(
                        one_d_cords(block, GRID_SIZE),
                        meshes.get_mut(mesh).unwrap(),
                        voxel_registry.as_ref(),
                    );
                }
            }
            if buttons.just_pressed(MouseButton::Left) {
                let (chunk, block, valid) = remove_block;
                if chunk == [0, 0] && valid {
                    grid.as_mut().remove_block(
                        one_d_cords(block, GRID_SIZE),
                        meshes.get_mut(mesh).unwrap(),
                        voxel_registry.as_ref(),
                    );
                }
            }
        }
    }
}

fn on_resize(
    mut resize_reader: EventReader<WindowResized>,
    mut commands: Commands,
    window: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(Entity, &Camera, &GlobalTransform), With<FlyCam>>,
) {
    for _ in resize_reader.read() {
        info!("window resize");
        let window = window.single();
        let (entity, camera, transform) = cams.single();
        commands
            .entity(entity)
            .remove::<RaycastSource<()>>()
            .insert(RaycastSource::<()>::new_screenspace(
                Vec2 {
                    x: window.width() / 1.66,
                    y: window.height() / 2.0,
                },
                camera,
                transform,
                window,
            ));
    }
}

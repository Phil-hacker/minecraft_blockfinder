use std::{
    borrow::Cow,
    mem::size_of,
    num::NonZeroU64,
    sync::mpsc::channel,
    time::{Duration, Instant},
};

use bevy::{
    prelude::*,
    render::{
        extract_resource::ExtractResourcePlugin,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::{
                storage_buffer_read_only_sized, storage_buffer_sized, uniform_buffer_sized,
            },
            *,
        },
        renderer::{RenderDevice, RenderQueue},
        MainWorld, RenderApp,
    },
};
use encase::internal::BufferRef;
use human_format::Scales;

use crate::{
    constants::{CHUNK_MARGIN, CHUNK_SIZE, GRID_SIZE, WORLD_HEIGHT},
    finder::{chunk::create_box, util::spiral},
    AppState,
};

use super::{FinderJob, FinderStatus};

#[derive(Resource)]
struct FindShaderData {
    find_bind_group: BindGroup,
    chunk_bind_group: BindGroup,
    position: UniformBuffer<IVec2>,
    result_gpu: StorageBuffer<UVec3>,
    result_cpu: Buffer,
    grid: StorageBuffer<Box<[u32; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2 / 4]>>,
    find_pipeline: CachedComputePipelineId,
    chunk_pipeline: CachedComputePipelineId,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct FinderLabel;

#[derive(Default)]
pub struct GPUFinderPlugin;

impl Plugin for GPUFinderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<FinderJob>::default());
        app.insert_resource(FinderStatus::WaitingForJob);
        app.add_systems(Update, update_label.run_if(in_state(AppState::Searching)));
        app.add_systems(OnEnter(AppState::Searching), init_searching_gui);
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(ExtractSchedule, copy_data);
        render_app.insert_resource(FinderStatus::WaitingForJob);
        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(FinderLabel, FindNode::default());
        render_graph.add_node_edge(FinderLabel, bevy::render::graph::CameraDriverLabel);
    }
    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<FindShaderData>();
    }
}

#[derive(Component)]
struct SearchingGui;

#[derive(Component)]
struct SearchedChunksLabel;

fn init_searching_gui(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: Color::srgb(0.15, 0.15, 0.15).into(),
                ..default()
            },
            SearchingGui,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle {
                    text: Text::from_section("0 blocks searched", TextStyle::default()),
                    ..default()
                },
                SearchedChunksLabel,
            ));
        });
    info!("gui setup complete")
}

fn update_label(
    mut label: Query<&mut Text, With<SearchedChunksLabel>>,
    finder_status: Res<FinderStatus>,
) {
    let mut formatter = human_format::Formatter::new();
    let mut scale = Scales::new();
    scale.with_suffixes(vec![
        "thousand",
        "million",
        "billion",
        "trillion",
        "quadrillion",
        "quintillion",
        "sextillion",
        "septillion",
    ]);
    formatter.with_scales(scale);
    match finder_status.as_ref() {
        FinderStatus::WaitingForJob => {}
        FinderStatus::Running { blocks, start_time } => {
            *label.single_mut().as_mut() = Text::from_section(
                format!(
                    "{} blocks searched\n{} seconds elapsed",
                    formatter.format(*blocks as f64),
                    start_time.elapsed().as_secs()
                ),
                TextStyle::default(),
            );
        }
        FinderStatus::Finished {
            searched_blocks,
            pos,
            time,
        } => {
            *label.single_mut().as_mut() = Text::from_section(
                format!(
                    "{} blocks searched\n{} seconds elapsed\n Found at: {}",
                    formatter.format(*searched_blocks as f64),
                    time.as_secs(),
                    pos
                ),
                TextStyle::default(),
            );
        }
    }
}

fn copy_data(mut main_world: ResMut<MainWorld>, finder_status: Res<FinderStatus>) {
    main_world.insert_resource(*finder_status.as_ref());
}

impl FromWorld for FindShaderData {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        // LAYOUTS
        let chunk_layout = chunk_layout(render_device);
        let find_layout = find_layout(render_device);
        // BUFFERS
        let mut chunk_size = UniformBuffer::from(UVec3::new(
            CHUNK_SIZE as u32,
            WORLD_HEIGHT as u32,
            CHUNK_SIZE as u32,
        ));
        let mut position = UniformBuffer::from(IVec2::splat(0));
        let mut result_gpu = StorageBuffer::from(UVec3::splat(u32::MAX));
        result_gpu.add_usages(BufferUsages::COPY_SRC);
        let result_cpu = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: UVec3::min_size().into(),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut grid = StorageBuffer::from(create_box::<
            u32,
            { GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2 / 4 },
        >());
        let mut chunk =
        StorageBuffer::from(create_box::<u32, { CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT / 16 }>());
        //  StorageBuffer::from(create_box::<u32, { CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT }>());
        chunk_size.write_buffer(render_device, render_queue);
        position.write_buffer(render_device, render_queue);
        result_gpu.write_buffer(render_device, render_queue);
        grid.write_buffer(render_device, render_queue);
        chunk.write_buffer(render_device, render_queue);
        let chunk_bind_group = render_device.create_bind_group(
            None,
            &chunk_layout,
            &BindGroupEntries::sequential((&position, &chunk)),
        );
        let find_bind_group = render_device.create_bind_group(
            None,
            &find_layout,
            &BindGroupEntries::sequential((&chunk_size, &chunk, &grid, &result_gpu)),
        );

        let find_shader = world.load_asset("shader://find.wgsl");
        let chunk_shader = world.load_asset("shader://chunk.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let find_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![find_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: find_shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });
        let chunk_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![chunk_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: chunk_shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });

        FindShaderData {
            find_bind_group,
            chunk_bind_group,
            position,
            result_gpu,
            result_cpu,
            grid,
            chunk_pipeline,
            find_pipeline,
        }
    }
}

fn chunk_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer_sized(false, NonZeroU64::new(size_of::<i32>() as u64 * 2)),
                storage_buffer_sized(
                    false,
                    NonZeroU64::new(
                        //(CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT * size_of::<u32>()) as u64,
                        (CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT * size_of::<u8>() / 4) as u64,
                    ),
                ),
            ),
        ),
    )
}

fn find_layout(render_device: &RenderDevice) -> BindGroupLayout {
    render_device.create_bind_group_layout(
        None,
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer_sized(false, NonZeroU64::new(size_of::<u32>() as u64 * 3)),
                storage_buffer_sized(
                    false,
                    NonZeroU64::new(
                        (CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT * size_of::<u8>() / 4) as u64,
                        //(CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT * size_of::<u32>()) as u64,
                    ),
                ),
                storage_buffer_read_only_sized(
                    false,
                    NonZeroU64::new(
                        (GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2 * size_of::<u8>()) as u64,
                    ),
                ),
                storage_buffer_sized(false, NonZeroU64::new(size_of::<u32>() as u64 * 3)),
            ),
        ),
    )
}

struct FindNode(FindNodeState, u32, Instant);

impl Default for FindNode {
    fn default() -> Self {
        Self(Default::default(), Default::default(), Instant::now())
    }
}

#[derive(Default)]
enum FindNodeState {
    #[default]
    LoadingPipelines,
    WaitingForTask,
    WaitingForGPU,
    ReadingData,
    Finished,
}

impl render_graph::Node for FindNode {
    fn run<'w>(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), render_graph::NodeRunError> {
        match self.0 {
            FindNodeState::LoadingPipelines | FindNodeState::WaitingForTask => {}
            FindNodeState::WaitingForGPU | FindNodeState::ReadingData => {
                let pipeline = world.resource::<FindShaderData>();
                let pipeline_cache = world.resource::<PipelineCache>();
                let chunk_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.chunk_pipeline)
                    .unwrap();
                let mut pass = render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor::default());
                pass.set_bind_group(0, &pipeline.chunk_bind_group, &[]);
                pass.set_pipeline(chunk_pipeline);
                pass.dispatch_workgroups(CHUNK_SIZE as u32 / 16, WORLD_HEIGHT as u32, CHUNK_SIZE as u32);
                //pass.dispatch_workgroups(CHUNK_SIZE as u32, WORLD_HEIGHT as u32, CHUNK_SIZE as u32);
                drop(pass);
                let find_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.find_pipeline)
                    .unwrap();
                let mut pass = render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor::default());
                pass.set_bind_group(0, &pipeline.find_bind_group, &[]);
                pass.set_pipeline(find_pipeline);
                pass.dispatch_workgroups(
                    (CHUNK_SIZE - CHUNK_MARGIN) as u32,
                    (WORLD_HEIGHT - GRID_SIZE.1) as u32,
                    (CHUNK_SIZE - CHUNK_MARGIN) as u32,
                );
                drop(pass);
                render_context.command_encoder().copy_buffer_to_buffer(
                    pipeline
                        .result_gpu
                        .buffer()
                        .expect("Buffer should have already been uploaded to the gpu"),
                    0,
                    &pipeline.result_cpu,
                    0,
                    (size_of::<u32>() as u64 * 3) as u64,
                );
            }
            FindNodeState::Finished => {}
        }
        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        match self.0 {
            FindNodeState::LoadingPipelines => {
                let pipeline = world.resource::<FindShaderData>();
                let pipeline_cache = world.resource::<PipelineCache>();
                match pipeline_cache.get_compute_pipeline_state(pipeline.chunk_pipeline) {
                    bevy::render::render_resource::CachedPipelineState::Ok(_) => {
                        match pipeline_cache.get_compute_pipeline_state(pipeline.find_pipeline) {
                            bevy::render::render_resource::CachedPipelineState::Ok(_) => {
                                self.0 = FindNodeState::WaitingForTask
                            }
                            bevy::render::render_resource::CachedPipelineState::Err(err) => {
                                panic!("shader error: {err:#?}")
                            }
                            _ => {}
                        }
                    }
                    bevy::render::render_resource::CachedPipelineState::Err(err) => {
                        panic!("shader error {err:#?}")
                    }
                    _ => {}
                }
            }
            FindNodeState::WaitingForTask => {
                if let Some(job) = world.get_resource::<FinderJob>() {
                    let grid = unsafe {
                        std::mem::transmute::<
                            Box<[u8; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2]>,
                            Box<[u32; GRID_SIZE.0 * GRID_SIZE.1 * GRID_SIZE.2 / 4]>,
                        >(job.0.clone())
                    };
                    world.resource_scope(|world, mut pipeline: Mut<FindShaderData>| {
                        let render_device = world.resource::<RenderDevice>();
                        let render_queue = world.resource::<RenderQueue>();
                        pipeline.grid.set(grid);
                        pipeline.grid.write_buffer(render_device, render_queue);
                    });
                    if let Some(mut finder_status) = world.get_resource_mut::<FinderStatus>() {
                        *finder_status.as_mut() = FinderStatus::Running {
                            blocks: 0,
                            start_time: Instant::now(),
                        };
                    }
                    set_position(world, self.1);
                    self.0 = FindNodeState::WaitingForGPU;
                    self.2 = Instant::now();
                }
            }
            FindNodeState::WaitingForGPU => {
                self.0 = FindNodeState::ReadingData;
            }
            FindNodeState::ReadingData => {
                world.resource_scope(|world, pipeline: Mut<FindShaderData>| {
                    let buffer = &pipeline.result_cpu;
                    let buffer_slice = buffer.slice(..);
                    let (sender, receiver) = channel();
                    buffer_slice.map_async(
                        bevy::render::render_resource::MapMode::Read,
                        move |v| match v {
                            Ok(_) => sender.send(()).unwrap(),
                            Err(err) => panic!("couldn't read data from gpu: {err:?}"),
                        },
                    );
                    if let Some(mut finder_status) = world.get_resource_mut::<FinderStatus>() {
                        if let FinderStatus::Running {
                            blocks,
                            start_time: _,
                        } = finder_status.as_mut()
                        {
                            *blocks += ((CHUNK_SIZE - CHUNK_MARGIN)
                                * (CHUNK_SIZE - CHUNK_MARGIN)
                                * WORLD_HEIGHT) as u64
                        }
                    };
                    let render_device = world.resource::<RenderDevice>();
                    render_device.poll(Maintain::wait()).panic_on_timeout();
                    receiver.recv().unwrap();
                    let buffer_view = buffer_slice.get_mapped_range();
                    let data: &[u8; 12] = buffer_view.read(0);
                    let data = unsafe { std::mem::transmute::<[u8; 12], UVec3>(*data) };
                    if data == UVec3::splat(u32::MAX) {
                        self.1 += 1;
                        self.0 = FindNodeState::ReadingData;
                    } else {
                        let (x, y) = spiral(self.1 as i32);
                        let (x, y) = (
                            x * (CHUNK_SIZE - CHUNK_MARGIN) as i32,
                            y * (CHUNK_SIZE - CHUNK_MARGIN) as i32,
                        );
                        let data = IVec3::new(data.x as i32 + x, data.y as i32, data.z as i32 + y);
                        info!("{data:?}, took {} seconds", self.2.elapsed().as_secs_f32());
                        self.0 = FindNodeState::Finished;
                        if let Some(mut finder_status) = world.get_resource_mut::<FinderStatus>() {
                            let mut blocks = 0;
                            let mut duration = Duration::default();
                            if let FinderStatus::Running {
                                blocks: v,
                                start_time,
                            } = finder_status.as_ref()
                            {
                                blocks = *v;
                                duration = start_time.elapsed();
                            }
                            *finder_status.as_mut() = FinderStatus::Finished {
                                searched_blocks: blocks,
                                pos: data,
                                time: duration,
                            }
                        }
                    }
                    drop(buffer_view);
                    buffer.unmap();
                });
                set_position(world, self.1);
            }
            FindNodeState::Finished => {}
        }
    }
}

fn set_position(world: &mut World, chunk_index: u32) {
    world.resource_scope(|world, mut pipeline: Mut<FindShaderData>| {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let (x, y) = spiral(chunk_index as i32);
        let pos = (
            x * (CHUNK_SIZE - CHUNK_MARGIN) as i32,
            y * (CHUNK_SIZE - CHUNK_MARGIN) as i32,
        )
            .into();
        info!("{pos}");
        pipeline.position.set(pos);
        pipeline.position.write_buffer(render_device, render_queue);
    });
}

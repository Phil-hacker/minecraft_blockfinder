use std::{env, fs::{self, create_dir_all, File}, io::{Read, copy}, path::{Path, PathBuf}};

#[allow(unused)]
use bevy::prelude::*;
use bevy::{asset::io::{AssetReader, AssetSourceBuilder, Reader}, render::{
    settings::{RenderCreation, WgpuFeatures, WgpuSettings},
    RenderPlugin,
}, tasks::futures_lite::{AsyncRead, AsyncSeek}};
use bevy_flycam::prelude::*;
use bevy_mod_raycast::prelude::*;
use finder::plugin::GPUFinderPlugin;
use zip::{read::ZipFile, ZipArchive};

pub mod block_list;
pub mod builder;
pub mod constants;
pub mod finder;
pub mod game_assets;
pub mod grid;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default, States)]
enum AppState {
    #[default]
    Loading,
    BuildingInit,
    Building,
    Searching,
    Finished,
}

fn main() {
    copy_minecraft_assets();
    let mut render_plugin = RenderPlugin::default();
    let mut wgpu_settings = WgpuSettings::default();
    wgpu_settings.features = wgpu_settings.features.union(WgpuFeatures::SHADER_INT64);
    println!("{:?}", wgpu_settings.features);
    render_plugin.render_creation = RenderCreation::Automatic(wgpu_settings);
    App::new()
        .register_asset_source(
            "shader",
            #[cfg(debug_assertions)]
            AssetSourceBuilder::platform_default("shaders", None),
            #[cfg(not(debug_assertions))]
            embedded_shader_source()
        )
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(render_plugin),
        )
        .add_plugins(PlayerPlugin)
        .init_state::<AppState>()
        .add_plugins(game_assets::GameAssetsPlugin)
        .add_plugins(grid::GridPlugin)
        .add_plugins(builder::BuilderPlugin)
        .add_plugins(DeferredRaycastingPlugin::<()>::default())
        .add_plugins(GPUFinderPlugin)
        .insert_resource(AmbientLight {
            brightness: 1250.0,
            color: Color::WHITE,
        })
        .run();
}

fn embedded_shader_source() -> AssetSourceBuilder {
    AssetSourceBuilder::default().with_reader(|| {
        Box::new(EmbeddedShaderReader)
    })
}

struct EmbeddedShaderReader;

impl AssetReader for EmbeddedShaderReader {
    fn read<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Box<bevy::asset::io::Reader<'a>>, bevy::asset::io::AssetReaderError>> {
        async {
            match path.to_str() {
                Some("find.wgsl") => {
                    let boxed: Box<bevy::asset::io::Reader> = Box::new(DataReader(include_bytes!("../shaders/find.wgsl")));
                    Ok(boxed)
                },
                Some("chunk.wgsl") => {
                    let boxed: Box<bevy::asset::io::Reader> = Box::new(DataReader(include_bytes!("../shaders/chunk.wgsl")));
                    Ok(boxed)
                },
                _ => {
                    Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned()))
                }
            }
        }
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Box<bevy::asset::io::Reader<'a>>, bevy::asset::io::AssetReaderError>> {
        async { Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned())) }
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Box<bevy::asset::io::PathStream>, bevy::asset::io::AssetReaderError>> {
        async { Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned())) }
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<bool, bevy::asset::io::AssetReaderError>> {
        async {
            match path.to_str() {
                Some("find.wgsl") | Some("chunk.wgsl") => {
                    Ok(false)
                },
                _ => {
                    Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned()))
                }
            }
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct DataReader(pub &'static [u8]);

impl AsyncRead for DataReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let read = self.get_mut().0.read(buf);
        std::task::Poll::Ready(read)
    }
}

impl AsyncSeek for DataReader {
    fn poll_seek(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: std::io::SeekFrom,
    ) -> std::task::Poll<std::io::Result<u64>> {
        std::task::Poll::Ready(Err(std::io::ErrorKind::Other.into()))
    }
}

fn copy_minecraft_assets() {
    let version_path = get_minecraft_version_path();
    let mut zip_file = ZipArchive::new(File::open(&version_path).expect("Couldn't load minecraft assets")).expect("Couldn't load minecraft assets");
    for index in 0..zip_file.len() {
        let mut file = zip_file.by_index(index).unwrap();
        let path = file.enclosed_name().unwrap();
        if path.starts_with("assets/") {
            if file.is_file() {
                create_dir_all(path.parent().unwrap()).unwrap();
                copy(&mut file,&mut File::create(path).unwrap()).unwrap();
            }
        }
    }
}

fn get_minecraft_version_path() -> PathBuf {
    let mut path = minecraft_folder_path::minecraft_dir().expect("Couldn't load minecraft assets");
    path.push("versions");
    let version = if let Ok(version) = env::var("MINECRAFT_VERSION") {
        path.push(format!("{0}/{0}.jar",version));
        path
    } else {
        fs::read_dir(&path).expect("Couldn't load minecraft assets").filter_map(|folder| {
            if let Ok(Some(data)) = folder.map(|folder| {
                let mut path = folder.path();
                let mut name = folder.file_name().to_string_lossy().to_string();
                name.push_str(".jar");
                path.push(name);
                if path.exists() && folder.file_name().to_string_lossy().to_string().starts_with("1.") && (13u32..=21).contains(&folder.file_name().to_string_lossy().to_string().split_terminator(".").nth(1).unwrap_or_default().parse::<u32>().unwrap_or_default()) {
                    Some(path)
                } else {
                    None
                }
            }) {
                Some(data)
            } else {
                None
            }
        }).max().expect("Couldn't load minecraft assets")
    };
    version
}

#[cfg(test)]
mod test {
    use crate::finder::util::{get_block_rotation, get_rendering_seed};

    #[test]
    fn test_generator() {
        let data: &[(i64, i64, i64, i64, u8)] = &include!("testdata.txt");
        for (x,y,z,seed,rotation) in data {
            assert_eq!(get_rendering_seed(*x, *y, *z), *seed);
            assert_eq!(get_block_rotation(*x, *y, *z), *rotation, "wrong rotation at {x},{y},{z}");
        }
    }
}
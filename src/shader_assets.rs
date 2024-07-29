use std::io::Read;

use bevy::{
    asset::io::{AssetReader, AssetSourceBuilder},
    tasks::futures_lite::{AsyncRead, AsyncSeek},
};

pub fn embedded_shader_source() -> AssetSourceBuilder {
    AssetSourceBuilder::default().with_reader(|| Box::new(EmbeddedShaderReader))
}

struct EmbeddedShaderReader;

impl AssetReader for EmbeddedShaderReader {
    fn read<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl bevy::utils::ConditionalSendFuture<
        Output = Result<Box<bevy::asset::io::Reader<'a>>, bevy::asset::io::AssetReaderError>,
    > {
        async {
            match path.to_str() {
                Some("find.wgsl") => {
                    let boxed: Box<bevy::asset::io::Reader> =
                        Box::new(DataReader(include_bytes!("../shaders/find.wgsl")));
                    Ok(boxed)
                }
                Some("chunk.wgsl") => {
                    let boxed: Box<bevy::asset::io::Reader> =
                        Box::new(DataReader(include_bytes!("../shaders/chunk.wgsl")));
                    Ok(boxed)
                }
                _ => Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned())),
            }
        }
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl bevy::utils::ConditionalSendFuture<
        Output = Result<Box<bevy::asset::io::Reader<'a>>, bevy::asset::io::AssetReaderError>,
    > {
        async { Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned())) }
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl bevy::utils::ConditionalSendFuture<
        Output = Result<Box<bevy::asset::io::PathStream>, bevy::asset::io::AssetReaderError>,
    > {
        async { Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned())) }
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<bool, bevy::asset::io::AssetReaderError>>
    {
        async {
            match path.to_str() {
                Some("find.wgsl") | Some("chunk.wgsl") => Ok(false),
                _ => Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned())),
            }
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct DataReader(pub &'static [u8]);

impl AsyncRead for DataReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let read = self.get_mut().0.read(buf);
        std::task::Poll::Ready(read)
    }
}

impl AsyncSeek for DataReader {
    fn poll_seek(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _pos: std::io::SeekFrom,
    ) -> std::task::Poll<std::io::Result<u64>> {
        std::task::Poll::Ready(Err(std::io::ErrorKind::Other.into()))
    }
}

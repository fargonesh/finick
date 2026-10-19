use freya_engine::prelude::Surface as SkiaSurface;
use smithay_client_toolkit::{
    reexports::client::{
        Dispatch,
        QueueHandle,
        protocol::{
            wl_buffer,
            wl_shm,
            wl_surface,
        },
    },
    shm::{
        Shm,
        raw::RawPool,
    },
};

use crate::drivers::raster::with_raster_surface;

#[derive(Debug)]
pub struct NativeWaylandError(pub(super) String);

impl std::fmt::Display for NativeWaylandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NativeWaylandError {}

impl From<String> for NativeWaylandError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

pub type NativeRenderCallback =
    Box<dyn FnMut((u32, u32), &mut SkiaSurface) -> Result<(), String> + 'static>;

const MAX_SHM_FRAME_BYTES: usize = 256 * 1024 * 1024;

pub(super) fn attach_shm_frame<D>(
    shm: &Shm,
    qh: &QueueHandle<D>,
    surface: &wl_surface::WlSurface,
    width: u32,
    height: u32,
    render: &mut NativeRenderCallback,
) -> Result<Option<String>, NativeWaylandError>
where
    D: Dispatch<wl_buffer::WlBuffer, ()> + 'static,
{
    if width == 0 || height == 0 {
        return Err(NativeWaylandError(
            "Compositor configured a zero-sized surface".into(),
        ));
    }
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err(NativeWaylandError(
            "Compositor configured a surface larger than Wayland SHM supports".into(),
        ));
    }
    if !shm.formats().contains(&wl_shm::Format::Argb8888) {
        return Err(NativeWaylandError(
            "Compositor does not support ARGB8888 SHM buffers".into(),
        ));
    }
    let stride = width
        .checked_mul(4)
        .ok_or_else(|| NativeWaylandError("SHM row size overflow".into()))?;
    if stride > i32::MAX as u32 {
        return Err(NativeWaylandError(
            "Compositor configured a row larger than Wayland SHM supports".into(),
        ));
    }
    let bytes = (stride as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| NativeWaylandError("SHM buffer size overflow".into()))?;
    if bytes > MAX_SHM_FRAME_BYTES || bytes > i32::MAX as usize {
        return Err(NativeWaylandError(
            "Compositor configured an SHM buffer above the 256 MiB limit".into(),
        ));
    }
    let mut pool = RawPool::new(bytes, shm)
        .map_err(|error| NativeWaylandError(format!("Failed to create SHM pool: {error}")))?;
    pool.mmap()
        .chunks_exact_mut(4)
        .for_each(|pixel| pixel.copy_from_slice(&[0, 0, 0, 255]));
    let render_result = with_raster_surface(pool.mmap(), width, height, |canvas| {
        render((width, height), canvas)
    })
    .map_err(|error| NativeWaylandError(error.into()))?;
    if render_result.is_err() {
        pool.mmap()
            .chunks_exact_mut(4)
            .for_each(|pixel| pixel.copy_from_slice(&[0, 0, 0, 255]));
    }

    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        wl_shm::Format::Argb8888,
        (),
        qh,
    );
    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, width as i32, height as i32);
    surface.commit();
    buffer.destroy();
    Ok(render_result.err())
}

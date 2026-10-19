use freya_engine::prelude::{AlphaType, ColorType, ImageInfo, Surface as SkiaSurface, wrap_pixels};

pub(crate) fn with_raster_surface<T>(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    render: impl FnOnce(&mut SkiaSurface) -> T,
) -> Result<T, &'static str> {
    let row_bytes = width as usize * 4;
    let info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );
    let mut surface =
        wrap_pixels(&info, pixels, Some(row_bytes), None).ok_or("Failed to wrap raster pixels")?;
    Ok(render(&mut surface))
}

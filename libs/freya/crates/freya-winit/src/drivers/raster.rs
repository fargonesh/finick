use freya_engine::prelude::{
    AlphaType,
    ColorType,
    ImageInfo,
    Surface as SkiaSurface,
    wrap_pixels,
};

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

#[cfg(test)]
mod tests {
    use super::with_raster_surface;

    #[test]
    fn software_surface_wraps_and_draws_into_caller_pixels() {
        let mut pixels = [0; 16];
        with_raster_surface(&mut pixels, 2, 2, |surface| {
            surface.canvas().clear(freya_core::prelude::Color::BLACK);
        })
        .unwrap();
        assert_eq!(
            pixels,
            [0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]
        );
    }
}

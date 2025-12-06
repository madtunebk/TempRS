use eframe::wgpu::{Device, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, Queue, TexelCopyBufferLayout};
use image::GenericImageView;

/// Load an image from a file path and create a WGPU texture
pub fn load_image_texture(device: &Device, queue: &Queue, path: &str) -> Result<(Texture, TextureView, [u32; 2]), String> {
    log::info!("Loading image texture from: {}", path);

    // Load image with image crate
    let img = image::open(path)
        .map_err(|e| format!("Failed to load image {}: {}", path, e))?;

    let rgba = img.to_rgba8();
    let dimensions = img.dimensions();

    log::info!("Image loaded: {}x{} pixels", dimensions.0, dimensions.1);

    // Create texture
    let texture_size = Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&TextureDescriptor {
        label: Some(&format!("image_texture_{}", path)),
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    // Write data to texture
    queue.write_texture(
        texture.as_image_copy(),
        &rgba,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        texture_size,
    );

    let view = texture.create_view(&TextureViewDescriptor::default());

    Ok((texture, view, [dimensions.0, dimensions.1]))
}

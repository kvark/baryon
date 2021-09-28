use std::{fs::File, io, path::Path};
use wgpu::util::DeviceExt as _;

impl super::Context {
    pub fn add_image_from_raw(
        &mut self,
        texture: wgpu::Texture,
        size: wgpu::Extent3d,
    ) -> super::ImageRef {
        let index = self.images.len();
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.images.push(super::Image { view, size });
        super::ImageRef(index as u32)
    }

    pub fn add_image_from_data(
        &mut self,
        desc: &wgpu::TextureDescriptor,
        data: &[u8],
    ) -> super::ImageRef {
        let texture = self
            .device
            .create_texture_with_data(&self.queue, desc, data);
        self.add_image_from_raw(texture, desc.size)
    }

    pub fn load_image(&mut self, path_ref: impl AsRef<Path>) -> super::ImageRef {
        let path = path_ref.as_ref();
        let image_format = image::ImageFormat::from_extension(path.extension().unwrap())
            .unwrap_or_else(|| panic!("Unrecognized image extension: {:?}", path.extension()));

        let label = path.display().to_string();
        let file = File::open(path)
            .unwrap_or_else(|e| panic!("Unable to open {}: {:?}", path.display(), e));
        let mut buf_reader = io::BufReader::new(file);

        let (texture, size) = if image_format == image::ImageFormat::Dds {
            let dds = ddsfile::Dds::read(&mut buf_reader)
                .unwrap_or_else(|e| panic!("Unable to read {}: {:?}", path.display(), e));

            println!("Header {:?}", dds.header);
            let mip_level_count = dds.get_num_mipmap_levels();
            let (dimension, depth_or_array_layers) = match dds.header10 {
                Some(ref h) => match h.resource_dimension {
                    ddsfile::D3D10ResourceDimension::Texture2D => {
                        (wgpu::TextureDimension::D2, h.array_size)
                    }
                    ddsfile::D3D10ResourceDimension::Texture3D => {
                        (wgpu::TextureDimension::D3, dds.get_depth())
                    }
                    other => panic!("Unsupported resource dimension {:?}", other),
                },
                None => match dds.header.depth {
                    None | Some(1) => (wgpu::TextureDimension::D2, 1),
                    Some(other) => (wgpu::TextureDimension::D3, other),
                },
            };

            let format = if let Some(fourcc) = dds.header.spf.fourcc {
                match fourcc.0 {
                    ddsfile::FourCC::BC1_UNORM => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
                    ddsfile::FourCC::BC2_UNORM => wgpu::TextureFormat::Bc2RgbaUnormSrgb,
                    ddsfile::FourCC::BC3_UNORM => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
                    ddsfile::FourCC::BC4_UNORM => wgpu::TextureFormat::Bc4RUnorm,
                    ddsfile::FourCC::BC4_SNORM => wgpu::TextureFormat::Bc4RSnorm,
                    ddsfile::FourCC::BC5_UNORM => wgpu::TextureFormat::Bc5RgUnorm,
                    ddsfile::FourCC::BC5_SNORM => wgpu::TextureFormat::Bc5RgSnorm,
                    ref other => panic!("Unsupported DDS FourCC {:?}", other),
                }
            } else {
                assert_eq!(dds.header.spf.rgb_bit_count, Some(32));
                wgpu::TextureFormat::Rgba8UnormSrgb
            };

            let desc = wgpu::TextureDescriptor {
                label: Some(&label),
                size: wgpu::Extent3d {
                    width: dds.header.width,
                    height: dds.header.height,
                    depth_or_array_layers,
                },
                mip_level_count,
                sample_count: 1,
                dimension,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
            };
            let texture = self
                .device
                .create_texture_with_data(&self.queue, &desc, &dds.data);

            (texture, desc.size)
        } else {
            let img = image::load(buf_reader, image_format)
                .unwrap_or_else(|e| panic!("Unable to decode {}: {:?}", path.display(), e))
                .to_rgba8();

            let (width, height) = img.dimensions();
            let size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            let desc = wgpu::TextureDescriptor {
                label: Some(&label),
                size,
                mip_level_count: 1, //TODO: generate `size.max_mips()` mipmaps
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            };
            let texture = self.device.create_texture(&desc);

            self.queue.write_texture(
                texture.as_image_copy(),
                &img,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: std::num::NonZeroU32::new(width * 4),
                    rows_per_image: None,
                },
                size,
            );
            (texture, size)
        };

        self.add_image_from_raw(texture, size)
    }
}

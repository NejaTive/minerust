use crate::pipelines::DEPTH_FORMAT;
use crate::shadow::ShadowQuality;

pub(crate) struct ShadowMap {
    pub quality: Option<ShadowQuality>,
    pub view: wgpu::TextureView,
    pub layers: Vec<wgpu::TextureView>,
}

impl ShadowMap {
    /// `None` — тени выключены: одиночный пустой слой, чтобы bind group оставалась полной.
    pub(crate) fn new(device: &wgpu::Device, quality: Option<ShadowQuality>) -> Self {
        let (size, layer_count) = quality.map_or((1, 1), |q| (q.resolution(), q.cascades() as u32));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: layer_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let layers = if quality.is_some() {
            (0..layer_count)
                .map(|layer| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: layer,
                        array_layer_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            quality,
            view,
            layers,
        }
    }
}

pub(crate) fn comparison_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("shadow"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    })
}

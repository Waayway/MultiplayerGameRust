use std::{io::{BufReader, Cursor}, path::Path};

use wgpu::util::DeviceExt;

use super::{texture, model::{self, MaterialUniform}};

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let txt = std::fs::read_to_string(path).unwrap();
    
    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = std::path::Path::new(env!("OUT_DIR"))
        .join("res")
        .join(file_name);
    let data = std::fs::read(path)?;

    Ok(data)
}

pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name)
}

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    let path = Path::new(file_name).parent().unwrap().to_str().unwrap();
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&format!("{}/{}", path, &p)).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;
    let mut materials = Vec::new();
    for mat in obj_materials? {
        let diffuse_path = mat.diffuse_texture;
        let diffuse_texture_w = if diffuse_path != "" {
            Some(load_texture(&format!("{}/{}", path, &diffuse_path), device, queue).await?)
        } else {
            None
        };
        let normal_path = mat.normal_texture;
        let normal_texture_w = if normal_path != "" {
            Some(load_texture(&format!("{}/{}", path, &normal_path), device, queue).await?)
        } else {
            None
        };

        let material_uniform = MaterialUniform {
            use_texture: if diffuse_texture_w.is_some() { 1 } else { 0 },
            _p1: [0.0; 3],
            ambient_color: mat.ambient.into(),
            _p2: 0,
            diffuse_color: mat.diffuse.into(),
            _p3: 0,
            specular_color: mat.specular.into(),
            _p4: 0,
        };

        let mat_uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Material Uniform Buffer"),
                contents: bytemuck::cast_slice(&[material_uniform]),
                usage: wgpu::BufferUsages::UNIFORM,
            }
        );

        let diffuse_texture = if diffuse_texture_w.is_some() {
            diffuse_texture_w.unwrap()
        } else {
            load_texture("assets/default_texture.png", device, queue).await?
        };
        let normal_texture = if normal_texture_w.is_some() {
            normal_texture_w.unwrap()
        } else {
            load_texture("assets/default_texture.png", device, queue).await?
        };

        let diffuse_texture_bind_group: wgpu::BindingResource = wgpu::BindingResource::TextureView(&diffuse_texture.view);
        let diffuse_sampler_bind_group: wgpu::BindingResource = wgpu::BindingResource::Sampler(&diffuse_texture.sampler);
        let normal_texture_bind_group: wgpu::BindingResource = wgpu::BindingResource::TextureView(&normal_texture.view);
        let normal_sampler_bind_group: wgpu::BindingResource = wgpu::BindingResource::Sampler(&normal_texture.sampler);
        let mat_uniform_bind_group: wgpu::BindingResource = mat_uniform_buffer.as_entire_binding();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: diffuse_texture_bind_group,
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: diffuse_sampler_bind_group,
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: normal_texture_bind_group,
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: normal_sampler_bind_group,
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: mat_uniform_bind_group,
                }
            ],
            label: None,
        });
        let diffuse_color = mat.diffuse;
        materials.push(model::Material {
            name: mat.name,
            diffuse_texture,
            normal_texture,
            diffuse_color,
            bind_group,
        })
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(model::Model { meshes, materials })
}

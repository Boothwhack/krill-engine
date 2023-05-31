use std::cell::{RefCell, RefMut};

use utils::Handle;

use crate::{BufferUsages, DeviceContext, MaybeOwned, VecBuf};
use crate::material::{UniformEntryDefinition, UniformEntryTypeDefinition};
use crate::render_api::DeviceResources;

pub struct Uniform {
    pub(crate) layout: Handle<wgpu::BindGroupLayout>,
    pub(crate) entries: Vec<UniformEntryDefinition>,
}

pub struct UniformInstance {
    layout: Handle<wgpu::BindGroupLayout>,
    entries: Vec<UniformInstanceEntry>,
    cache: RefCell<UniformCache>,
}

pub(crate) struct UniformCache {
    signature: Vec<EntrySignature>,
    bind_group: wgpu::BindGroup,
}

impl UniformCache {
    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

enum EntrySignature {
    Buffer(u32),
}

impl UniformInstance {
    pub fn new(device: &DeviceContext, resources: &DeviceResources, uniform: &Uniform, values: Vec<Option<UniformInstanceEntry>>) -> Self {
        let entries: Vec<_> = uniform.entries.iter().zip(values)
            .map(|(def, value)| match value {
                Some(value) => value,
                None => match def.typ {
                    UniformEntryTypeDefinition::Buffer => UniformInstanceEntry::Buffer(
                        MaybeOwned::from(device.create_buffer(0, BufferUsages::UNIFORM | BufferUsages::COPY_DST))
                    ),
                }
            })
            .collect();

        let cache = Self::cache_entries(device, resources, &entries, uniform.layout);

        UniformInstance {
            layout: uniform.layout,
            entries,
            cache: RefCell::new(cache),
        }
    }

    pub(crate) fn cache(&self) -> RefMut<'_, UniformCache> {
        self.cache.borrow_mut()
    }

    fn cache_entries(device: &DeviceContext, resources: &DeviceResources, entries: &[UniformInstanceEntry], layout: Handle<wgpu::BindGroupLayout>) -> UniformCache {
        let (entry_bindings, signature): (Vec<_>, Vec<_>) = entries.iter()
            .enumerate()
            .map(|(i, entry)| {
                let (resource, signature) = match entry {
                    UniformInstanceEntry::Buffer(MaybeOwned::Handle(buffer)) => {
                        let buffer = resources.buffers.get(*buffer).unwrap();
                        (buffer.buffer.as_entire_binding(), EntrySignature::Buffer(buffer.version()))
                    }
                    UniformInstanceEntry::Buffer(MaybeOwned::Owned(buffer)) => {
                        (buffer.buffer.as_entire_binding(), EntrySignature::Buffer(buffer.version()))
                    }
                };
                (wgpu::BindGroupEntry {
                    binding: i as _,
                    resource,
                }, signature)
            })
            .unzip();
        let layout = resources.bind_group_layouts.get(layout).unwrap();
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Default::default(),
            entries: &entry_bindings,
            layout,
        });
        UniformCache {
            signature,
            bind_group,
        }
    }

    fn test_signature(&self, resources: &DeviceResources) -> bool {
        self.cache.borrow().signature.iter().zip(self.entries.iter()).all(|(signature, entry)| match signature {
            EntrySignature::Buffer(version) => match entry {
                UniformInstanceEntry::Buffer(MaybeOwned::Owned(buffer)) => buffer.version() == *version,
                UniformInstanceEntry::Buffer(MaybeOwned::Handle(buffer)) => {
                    let buffer = resources.buffers.get(*buffer).unwrap();
                    buffer.version() == *version
                }
                _ => false,
            }
        })
    }

    pub(crate) fn validate_bind_group(&self, device: &DeviceContext, resources: &DeviceResources) {
        if !self.test_signature(resources) {
            self.cache.replace(Self::cache_entries(device, resources, &self.entries, self.layout));
        }
    }

    pub fn entries(&self) -> &[UniformInstanceEntry] {
        self.entries.as_slice()
    }
}


pub enum UniformInstanceEntry {
    Buffer(MaybeOwned<VecBuf>),
}

impl UniformInstanceEntry {
    fn matches_definition(&self, entry: &UniformEntryDefinition) -> bool {
        match self {
            UniformInstanceEntry::Buffer(_) => matches!(entry.typ, UniformEntryTypeDefinition::Buffer),
        }
    }
}

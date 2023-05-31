use wgpu::BufferAddress;

use crate::{BufferUsages, MutableHandle};

/// Resizable wrapper for [wgpu::Buffer].
pub struct VecBuf {
    pub(crate) buffer: wgpu::Buffer,
    version: u32,
    size: usize,
    capacity: usize,
    usage: BufferUsages,
}

impl VecBuf {
    pub(crate) fn new(buffer: wgpu::Buffer, capacity: usize, usage: BufferUsages) -> Self {
        VecBuf {
            buffer,
            version: 0,
            size: 0,
            capacity,
            usage,
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn usage(&self) -> BufferUsages {
        self.usage
    }

    pub fn entire_slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(0..self.size as _)
    }
}

impl<'a> MutableHandle<'a, VecBuf> {
    /// Ensures that this buffer has capacity at least of size. Returns whether the buffer was
    /// recreated, in which case it may be mapped if the `mapped` parameter is passed true.
    pub fn set_capacity_at_least(&mut self, size: usize, mapped: bool) -> bool {
        // buffer might be regenerated, assume data is erased
        self.clear();

        if self.resource.capacity < size {
            let size = size as wgpu::BufferAddress;
            let size = size + size % wgpu::COPY_BUFFER_ALIGNMENT;
            self.resource.buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
                label: wgpu::Label::default(),
                size,
                usage: self.resource.usage,
                mapped_at_creation: mapped,
            });
            self.resource.version += 1;
            self.resource.capacity = size as _;

            true
        } else {
            false
        }
    }

    /// Destructively uploads new data to this buffer. Old data may remain if the new data is
    /// smaller than the buffer's capacity.
    pub fn upload(&mut self, offset: usize, data: &[u8]) {
        if self.set_capacity_at_least(offset + data.len(), true) {
            let mut view = self.resource.buffer
                .slice(offset as BufferAddress..offset as BufferAddress + data.len() as BufferAddress)
                .get_mapped_range_mut();
            view.copy_from_slice(data);
            self.resource.buffer.unmap();
        } else {
            self.context.queue.write_buffer(&self.resource.buffer, offset as _, data);
        }
    }

    /// Pushes data to the end of the buffers data. Panics if the operation would exceed the
    /// buffer's capacity.
    pub fn push(&mut self, data: &[u8]) {
        if self.resource.size + data.len() > self.resource.capacity {
            panic!("buffer capacity overflow");
        }
        self.context.queue.write_buffer(&self.resource.buffer, self.resource.size as _, data);
        self.resource.size += data.len();
    }

    pub fn clear(&mut self) {
        self.resource.size = 0;
    }
}

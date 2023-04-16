use std::future::Future;
use frunk::HNil;
use crate::resource::{ResourceList, Resources};

pub struct ProcessInfo;

pub struct ProcessBuilder<R: ResourceList> {
    resources: R,
}

impl ProcessBuilder<Resources<ProcessInfo, HNil>> {
    pub fn new(process_info: ProcessInfo) -> Self {
        ProcessBuilder { resources: Resources::new(process_info) }
    }
}

impl<R: ResourceList> ProcessBuilder<R> {
    pub fn setup<R2, F>(self, setup: F) -> ProcessBuilder<R2>
        where R2: ResourceList,
              F: FnOnce(R) -> R2, {
        ProcessBuilder { resources: setup(self.resources) }
    }

    pub fn build(self) -> R {
        self.resources
    }
}

impl<R: ResourceList> ProcessBuilder<R> {
    pub async fn setup_async<R2, F, Fut>(self, setup: F) -> ProcessBuilder<R2>
        where R2: ResourceList,
              F: FnOnce(R) -> Fut,
              Fut: Future<Output=R2> {
        ProcessBuilder { resources: setup(self.resources).await }
    }
}

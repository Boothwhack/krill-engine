use std::future::{IntoFuture};
use async_trait::async_trait;
use utils::hlist::{Concat, IntoShape};

pub struct ProcessInfo;

pub struct ProcessBuilder<R> {
    resources: R,
}

impl ProcessBuilder<()> {
    pub fn new() -> ProcessBuilder<()> {
        ProcessBuilder { resources: () }
    }
}

pub trait ProcessSetupStep {
    type Input;
    type Output;

    fn setup(self, input: Self::Input) -> Self::Output;
}

pub trait ProcessBuilderExt<S: ProcessSetupStep, I> {
    type Output;
}

#[async_trait(? Send)]
pub trait AsyncProcessSetupStep {
    type Input;
    type Output;

    async fn setup(self, input: Self::Input) -> Self::Output;
}

impl<R> ProcessBuilder<R> {
    pub fn setup<F, Input, InputI, Output>(self, setup: F) -> ProcessBuilder<<R::Remainder as Concat>::Concatenated<Output>>
        where R: IntoShape<Input, InputI>,
              R::Remainder: Concat,
              F: FnOnce(Input) -> Output {
        let (input, remainder) = self.resources.into_shape();
        let output = setup(input);
        let resources = remainder.concat(output);
        ProcessBuilder { resources }
    }

    pub async fn setup_async<F, Input, InputI, Output, Fut>(self, setup: F) -> ProcessBuilder<<R::Remainder as Concat>::Concatenated<Output>>
        where R: IntoShape<Input, InputI>,
              R::Remainder: Concat,
              Fut: IntoFuture<Output=Output>,
              F: FnOnce(Input) -> Fut {
        let (input, remainder) = self.resources.into_shape();
        let output = setup(input).await;
        let resources = remainder.concat(output);
        ProcessBuilder { resources }
    }

    pub fn build(self) -> R {
        self.resources
    }
}

#[cfg(test)]
mod tests {
    use utils::{hlist, HList};
    use utils::hlist::Has;
    use crate::process::ProcessBuilder;

    #[test]
    fn setup() {
        let list = ProcessBuilder::new()
            .setup(|_| hlist!(25u32))
            .setup(|i: HList!(u32)| hlist!(*i.get(), "string".to_owned()))
            .setup(|s: HList!(String)| hlist!(0.6f32, false))
            .build();
        assert_eq!(hlist!(25u32, 0.6f32, false), list);
    }
}

use frunk::hlist::{HList, Plucker, Sculptor, Selector};
pub use frunk::HList;
use frunk::{HCons, hlist, HNil, ToMut, ToRef};

// wraps a non-empty HList
pub struct Resources<H, T: HList> {
    resources: HList!(H, ...T),
}

impl<H> Resources<H, HNil> {
    pub fn new(resource: H) -> Resources<H, HNil> {
        Resources { resources: hlist![resource] }
    }
}

pub trait ResourceList {
    type Resources: HList;
    type WithResource<R>: ResourceList<Resources=HList!(R, ...Self::Resources)>;

    fn with_resource<R>(self, resource: R) -> Self::WithResource<R>;

    fn list(&self) -> &Self::Resources;

    fn list_mut(&mut self) -> &mut Self::Resources;

    fn unpack(self) -> Self::Resources;

    fn get<R, I>(&self) -> &R
        where Self::Resources: Selector<R, I> {
        self.list().get()
    }

    fn get_mut<R, I>(&mut self) -> &mut R
        where Self::Resources: Selector<R, I> {
        self.list_mut().get_mut()
    }

    fn to_ref<'a>(&'a self) -> <Self::Resources as ToRef>::Output
        where Self::Resources: ToRef<'a> {
        self.list().to_ref()
    }

    fn to_mut<'a, R, I>(&'a mut self) -> R
        where Self::Resources: ToMut<'a>,
              <Self::Resources as ToMut<'a>>::Output: Sculptor<R, I> {
        self.list_mut().to_mut().sculpt().0
    }
}

impl<Head, Tail: HList> ResourceList for Resources<Head, Tail> {
    type Resources = HList!(Head, ...Tail);
    type WithResource<R> = Resources<R, Self::Resources>;

    fn with_resource<R>(self, resource: R) -> Resources<R, Self::Resources> {
        Resources { resources: self.resources.prepend(resource) }
    }

    fn list(&self) -> &Self::Resources {
        &self.resources
    }

    fn list_mut(&mut self) -> &mut Self::Resources {
        &mut self.resources
    }

    fn unpack(self) -> Self::Resources {
        self.resources
    }
}

pub trait ResourceListHas<S, I>: ResourceList {}

impl<T, S, I> ResourceListHas<S, I> for T
    where T: ResourceList,
          T::Resources: Selector<S, I> {}

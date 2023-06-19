use utils::Handle;

pub enum MaybeOwned<T> {
    Handle(Handle<T>),
    Owned(T),
}

pub enum MaybeRef<'a, T> {
    Handle(Handle<T>),
    Ref(&'a mut T),
}

impl<T> From<Handle<T>> for MaybeOwned<T> {
    fn from(value: Handle<T>) -> Self {
        MaybeOwned::Handle(value)
    }
}

impl<T> From<T> for MaybeOwned<T> {
    fn from(value: T) -> Self {
        MaybeOwned::Owned(value)
    }
}

impl<'a, T> From<Handle<T>> for MaybeRef<'a, T> {
    fn from(value: Handle<T>) -> Self {
        MaybeRef::Handle(value)
    }
}

impl<'a, T> From<&'a mut T> for MaybeRef<'a, T> {
    fn from(value: &'a mut T) -> Self {
        MaybeRef::Ref(value)
    }
}

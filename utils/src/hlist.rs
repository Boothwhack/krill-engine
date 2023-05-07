#![allow(non_snake_case)]

mod has;
mod map;
mod shape;

pub use has::*;
pub use map::*;
pub use shape::*;

pub trait Prepend {
    fn prepend<T>(self, value: T) -> (T, Self);
}

impl Prepend for () {
    fn prepend<T>(self, value: T) -> (T, Self) {
        (value, ())
    }
}

impl<Head, Tail> Prepend for (Head, Tail) {
    fn prepend<T>(self, value: T) -> (T, Self) {
        (value, self)
    }
}

pub trait Concat {
    type Concatenated<T>;

    fn concat<T>(self, list: T) -> Self::Concatenated<T>;
}

impl Concat for () {
    type Concatenated<T> = T;

    fn concat<T>(self, list: T) -> Self::Concatenated<T> {
        list
    }
}

impl<Head, Tail> Concat for (Head, Tail)
    where Tail: Concat {
    type Concatenated<T> = (Head, Tail::Concatenated<T>);

    fn concat<T>(self, list: T) -> Self::Concatenated<T> {
        (self.0, self.1.concat(list))
    }
}

pub trait ToRef {
    type Output<'a> where Self: 'a;

    fn to_ref(&self) -> Self::Output<'_>;
}

impl ToRef for () {
    type Output<'a> = ()
        where Self: 'a;

    fn to_ref(&self) -> Self::Output<'_> {
        ()
    }
}

impl<Head, Tail> ToRef for (Head, Tail)
    where Tail: ToRef {
    type Output<'a> = (&'a Head, Tail::Output<'a>)
        where Self: 'a;

    fn to_ref(&self) -> Self::Output<'_> {
        (&self.0, self.1.to_ref())
    }
}

pub trait ToMut {
    type Output<'a> where Self: 'a;

    fn to_mut(&mut self) -> Self::Output<'_>;
}

impl ToMut for () {
    type Output<'a> = ()
        where Self: 'a;

    fn to_mut(&mut self) -> Self::Output<'_> {
        ()
    }
}

impl<Head, Tail> ToMut for (Head, Tail)
    where Tail: ToMut {
    type Output<'a> = (&'a mut Head, Tail::Output<'a>)
        where Self: 'a;

    fn to_mut(&mut self) -> Self::Output<'_> {
        (&mut self.0, self.1.to_mut())
    }
}

#[macro_export]
macro_rules! hlist {
    () => { () };
    ($head:expr $(,)?) => {
        ($head, hlist!())
    };
    ($head:expr, $($tail:expr),* $(,)?) => {
        ($head, hlist!($($tail),*))
    };
}

/// Macro for destructuring an hlist.
#[macro_export]
macro_rules! delist {
    ($head:pat $(,)?) => { ($head,_) };
    ($head:pat, $($tail:pat),* $(,)?) => {
        ($head, delist!($($tail),*))
    };
}

#[macro_export]
macro_rules! HList {
    () => {
        ()
    };
    ($head:ty $(,)?) => {
        ($head, HList!())
    };
    ($head:ty, $($tail:ty),* $(,)?) => {
        ($head, HList!($($tail),*))
    };
}

#[cfg(test)]
mod tests {
    use crate::hlist::{Concat, IntoShape};

    #[test]
    fn macros() {
        let empty_list: HList!() = hlist!();
        assert_eq!(empty_list, ());

        let single_list: HList!(String) = hlist!("Hi".to_owned());
        assert_eq!(single_list, ("Hi".to_owned(), ()));

        let list: HList!(u32, f64, bool) = hlist!(10u32, 1.8f64, true);
        assert_eq!(list, (10u32, (1.8f64, (true, ()))));

        let three_list: HList!(u32, f32, bool) = hlist!(25u32, 0.6f32, false);
        assert_eq!(three_list, (25u32, (0.6f32, (false, ()))));
    }

    struct Nesting {
        value: String,
    }

    #[test]
    fn destruct() {
        let list = hlist!(12u32, 9.7f32, false);
        let delist!(variable) = list;
        assert_eq!(12u32, variable);

        let delist!(int, float, boolean) = list;
        assert_eq!(12u32, int);
        assert_eq!(9.7f32, float);
        assert_eq!(false, boolean);

        let delist!(mut int) = list;
        assert_eq!(12u32, int);
        int = 13u32;
        assert_eq!(13u32, int);

        let list = hlist!(Nesting {value: "Hello".to_owned()});
        let delist!(Nesting{value}) = list;
        assert_eq!("Hello".to_owned(), value);
    }

    #[test]
    fn shape() {
        let list = hlist!(10u32, "string", 2.5f32, false);

        let (sub_list, remainder): (HList!(&str, bool), _) = list.into_shape();
        assert_eq!(sub_list, hlist!("string", false));
        assert_eq!(remainder, hlist!(10u32, 2.5f32));
    }

    #[test]
    fn concat() {
        let root_list = hlist!("string", 23u32);
        let second_list = hlist!(false, 2.5f32);
        let concatenated = root_list.concat(second_list);
        assert_eq!(concatenated, hlist!("string", 23u32, false, 2.5f32));
    }

    #[test]
    fn shape_concat() {
        let full_list = hlist!(25u32, "string".to_owned());
        let (shaped, remainder): (HList!(String), _) = full_list.into_shape();
        assert_eq!(hlist!("string".to_owned()), shaped);
        let merged = remainder.concat(hlist!(0.6f32, false));
        assert_eq!(hlist!(25u32, 0.6f32, false), merged);
    }
}

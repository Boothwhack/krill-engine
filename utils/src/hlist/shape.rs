use crate::hlist::{Has};

pub trait IntoShape<Shape, Indices> {
    type Remainder;

    fn into_shape(self) -> (Shape, Self::Remainder);
}

impl<L> IntoShape<(), ()> for L {
    type Remainder = L;

    fn into_shape(self) -> ((), Self::Remainder) {
        ((), self)
    }
}

impl<L, ShapeHead, ShapeTail, HeadIndex, TailIndex>
IntoShape<(ShapeHead, ShapeTail), (HeadIndex, TailIndex)> for L
    where L: Has<ShapeHead, HeadIndex>,
          <L as Has<ShapeHead, HeadIndex>>::Remainder: IntoShape<ShapeTail, TailIndex> {
    type Remainder = <<Self as Has<ShapeHead, HeadIndex>>::Remainder as IntoShape<ShapeTail, TailIndex>>::Remainder;

    fn into_shape(self) -> ((ShapeHead, ShapeTail), Self::Remainder) {
        let (picked, remainder) = self.pick();
        let (tail_shaped, tail_remainder) = remainder.into_shape();
        ((picked, tail_shaped), tail_remainder)
    }
}

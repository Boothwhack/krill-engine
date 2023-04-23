use std::marker::PhantomData;

/// Marker for query operations
pub struct Here;

/// Marker for query operations
pub struct There<T> {
    phantom_data: PhantomData<T>,
}

pub trait Has<T, I>: Sized {
    type Remainder;

    fn get(&self) -> &T;

    fn get_mut(&mut self) -> &mut T;

    fn pick(self) -> (T, Self::Remainder);
}

impl<Head, Tail> Has<Head, Here> for (Head, Tail) {
    type Remainder = Tail;

    fn get(&self) -> &Head {
        &self.0
    }

    fn get_mut(&mut self) -> &mut Head {
        &mut self.0
    }

    fn pick(self) -> (Head, Self::Remainder) {
        self
    }
}

impl<Head, Tail, FromTail, TailIndex> Has<FromTail, There<TailIndex>> for (Head, Tail)
    where Tail: Has<FromTail, TailIndex> {
    type Remainder = (Head, Tail::Remainder);

    fn get(&self) -> &FromTail {
        self.1.get()
    }

    fn get_mut(&mut self) -> &mut FromTail {
        self.1.get_mut()
    }

    fn pick(self) -> (FromTail, Self::Remainder) {
        let (picked, remainder) = self.1.pick();
        (picked, (self.0, remainder))
    }
}

#[cfg(test)]
mod tests {
    use crate::hlist;
    use crate::hlist::Has;

    #[test]
    fn get() {
        let list = hlist!(10u32, "str", 0.0f32);
        {
            let string: &&str = list.get();
            assert_eq!(*string, "str");
        }
        {
            let uint32: &u32 = list.get();
            assert_eq!(*uint32, 10u32);
        }
        {
            let float32: &f32 = list.get();
            assert_eq!(*float32, 0.0);
        }
    }
}

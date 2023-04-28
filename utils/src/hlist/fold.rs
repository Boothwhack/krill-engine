use std::ops::Try;

pub trait FnFoldHList<L, B, R> {
    fn invoke(self, accum: B, list: L) -> R;
}

impl<Head, Tail, LHead, LTail, B, R> FnFoldHList<(LHead, LTail), B, R> for (Head, Tail)
    where Head: FnMut(B, LHead) -> R,
          Tail: FnFoldHList<LTail, B, R>,
          R: Try<Output=B> {
    fn invoke(self, accum: B, list: (LHead, LTail)) -> R {
        let (mut head, tail) = self;
        let accum = head(accum, list.0)?;
        tail.invoke(accum, list.1)
    }
}

impl<B, R> FnFoldHList<(), B, R> for ()
    where R: Try<Output=B> {
    fn invoke(self, accum: B, _list: ()) -> R {
        R::from_output(accum)
    }
}

pub trait TryFold {
    fn try_fold<B, F, R>(self, initial: B, f: F) -> R
        where Self: Sized,
              F: FnFoldHList<Self, B, R>,
              R: Try<Output=B>;
}

impl<Head, Tail> TryFold for (Head, Tail) {
    fn try_fold<B, F, R>(self, accum: B, f: F) -> R
        where Self: Sized,
              F: FnFoldHList<Self, B, R>,
              R: Try<Output=B> {
        f.invoke(accum, self)
    }
}

#[cfg(test)]
mod tests {
    use crate::hlist;
    use crate::hlist::TryFold;
    use std::str::FromStr;

    #[test]
    fn try_fold() {
        let folder = hlist!(
            |a, i| {Some(a + i as f32)},
            |a, s: &str| {Some(a + f32::from_str(s).ok()?)},
            |a, f| {Some(a + f)},
        );

        let valid_list = hlist!(12u32, "4", 4.5f32);
        let result = valid_list.try_fold(0.0f32, folder);
        assert_eq!(Some(12.0 + 4.0 + 4.5), result);

        let invalid_list = hlist!(12u32, "this is not a number", 4.5f32);
        let result = invalid_list.try_fold(0.0f32, folder);
        assert_eq!(None, result);
    }
}

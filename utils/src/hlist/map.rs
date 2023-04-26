pub trait FnMapHList<L, R> {
    fn invoke(self, list: L) -> R;
}

impl<Head, Tail, LHead, LTail, RHead, RTail> FnMapHList<(LHead, LTail), (RHead, RTail)> for (Head, Tail)
    where Head: FnMut(LHead) -> RHead,
          Tail: FnMapHList<LTail, RTail> {
    fn invoke(self, list: (LHead, LTail)) -> (RHead, RTail) {
        let (mut head, tail) = self;
        (head(list.0), tail.invoke(list.1))
    }
}

impl FnMapHList<(), ()> for () {
    fn invoke(self, _list: ()) -> () {
        ()
    }
}

pub trait Mappable {
    fn map<F, R>(self, f: F) -> R
        where Self: Sized,
              F: FnMapHList<Self, R>;
}

impl<Head, Tail> Mappable for (Head, Tail) {
    fn map<F, R>(self, f: F) -> R
        where Self: Sized,
              F: FnMapHList<Self, R> {
        f.invoke(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::hlist;
    use crate::hlist::Mappable;

    #[test]
    fn map() {
        let list_a = hlist!(5u32, 1.8f32, true);
        let list_b = list_a.map(hlist!(
            |int: u32| (int as f32) * 1.5,
            |float: f32| float.round() as u32,
            |boolean: bool| if boolean {"True".to_owned()} else {"False".to_owned()}
        ));
        assert_eq!(hlist!(7.5f32, 2u32, "True".to_owned()), list_b);
    }
}

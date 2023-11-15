use utils::hlist::{ToMut, Has, IntoShape};

pub struct Resources<R> {
    resource_list: R,
}

impl<R> Resources<R> {
    pub fn new(resource_list: R) -> Self {
        Resources { resource_list }
    }

    pub fn get<T, I>(&mut self) -> &mut T
        where R: Has<T, I> {
        self.resource_list.get_mut()
    }

    pub fn get_some<SR, SI>(&mut self) -> SR::Output<'_>
        where R: ToMut,
              SR: ToMut,
              for<'a> R::Output<'a>: IntoShape<SR::Output<'a>, SI> {
        self.resource_list.to_mut().into_shape().0
    }
}

pub trait HasResources<SR, SI>
    where SR: ToMut {
    fn res(&mut self) -> SR::Output<'_>;
}

impl<SR, SI, R> HasResources<SR, SI> for Resources<R>
    where R: ToMut,
          SR: ToMut,
          for<'a> R::Output<'a>: IntoShape<SR::Output<'a>, SI>, {
    fn res(&mut self) -> <SR as ToMut>::Output<'_> {
        self.resource_list.to_mut().into_shape().0
    }
}

#[cfg(test)]
mod tests {
    use utils::{hlist, HList, delist};

    use crate::resources::HasResources;

    use super::Resources;

    struct ResourceA(u32);

    struct ResourceB(f32);

    struct ResourceC(&'static str);

    #[test]
    fn get_resources_individually() {
        let mut res = Resources::new(hlist!(ResourceA(10u32), ResourceB(0.7f32), ResourceC("Hello")));

        {
            let res_a: &mut ResourceA = res.get();
            assert_eq!(res_a.0, 10u32);
        }
        {
            let res_b: &mut ResourceB = res.get();
            assert_eq!(res_b.0, 0.7f32);
        }
        {
            let res_c: &mut ResourceC = res.get();
            assert_eq!(res_c.0, "Hello");
        }
    }

    #[test]
    fn get_resources_bulk() {
        let mut res = Resources::new(hlist!(ResourceA(10u32), ResourceB(0.7f32), ResourceC("Hello")));

        {
            let delist!(res_a, res_c) = res.get_some::<HList!(ResourceA, ResourceC), _>();
            assert_eq!(res_a.0, 10u32);
            assert_eq!(res_c.0, "Hello");
        }
    }

    #[test]
    fn has_resources_trait() {
        fn takes_resources<R, SI>(resources: &mut Resources<R>) -> bool
            where Resources<R>: HasResources<HList!(ResourceC, ResourceB), SI> {
            let delist!(res_c, res_b) = resources.res();

            assert_eq!(res_c.0, "Hello");
            assert_eq!(res_b.0, 0.7f32);

            true
        }

        {
            let mut res = Resources::new(hlist!(ResourceA(10u32), ResourceB(0.7f32), ResourceC("Hello")));
            assert!(takes_resources(&mut res));
        }
        {
            let mut res = Resources::new(hlist!(ResourceC("Hello"), ResourceB(0.7f32)));
            assert!(takes_resources(&mut res));
        }
    }
}

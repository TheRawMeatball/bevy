use std::sync::Arc;

use downcast_rs::impl_downcast;
use parking_lot::Mutex;

use crate::{FetchApplyable, Resources, SystemParam, World};

pub trait Applyable: Send + Sync + downcast_rs::Downcast {
    fn apply(&mut self, world: &mut World, resources: &mut Resources);
}

pub trait ApplyableExt<'a> {
    type Front: ApplyableFront<'a, Backend = Self>;

    fn get_front(&'a mut self) -> Self::Front;
    fn new(world: &World, resources: &mut Resources) -> Self;
}

pub trait ApplyableFront<'a>: Send + Sync + Sized + 'a {
    type Backend: Applyable + ApplyableExt<'a, Front = Self>;
}

impl<T: Applyable> Applyable for Arc<Mutex<T>> {
    fn apply(&mut self, world: &mut World, resources: &mut Resources) {
        self.lock().apply(world, resources);
    }
}

impl<T: Applyable + ApplyableExt<'static>> ApplyableExt<'static> for Arc<Mutex<T>> {
    type Front = Self;

    fn get_front(&mut self) -> Self::Front {
        self.clone()
    }

    fn new(world: &World, resources: &mut Resources) -> Self {
        let t = T::new(world, resources);
        Arc::new(Mutex::new(t))
    }
}

impl<T: Applyable + ApplyableExt<'static>> ApplyableFront<'static> for Arc<Mutex<T>> {
    type Backend = Self;
}

impl<'a, T, X> ApplyableFront<'a> for T
where
    T: SystemParam<Fetch = FetchApplyable<X>> + Send + Sync + 'a,
    X: Applyable + ApplyableExt<'a, Front = T>,
{
    type Backend = X;
}

impl_downcast!(Applyable);

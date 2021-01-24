use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Weak,
};

use bevy_ecs::{Applyable, Local, Res, ResMut, Resource, SystemParam};
use bevy_utils::HashMap;

use crate::{Events, ManualEventReader};

pub struct Channels<T: Resource> {
    next_id: AtomicUsize,
    map: HashMap<usize, (Events<T>, Weak<()>)>,
}

impl<T: Resource> Default for Channels<T> {
    fn default() -> Self {
        Self {
            next_id: 0.into(),
            map: Default::default(),
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Id {
    id: usize,
    ref_counter: Arc<()>,
}

impl<T: Resource> Channels<T> {
    pub fn reserve(&self) -> Id {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let ref_counter = Arc::new(());

        let id = Id { id, ref_counter };
        id
    }

    fn events_usize(&self, id: &usize) -> &Events<T> {
        &self.map.get(id).unwrap().0
    }

    fn events_mut_usize(&mut self, id: &usize) -> &mut Events<T> {
        &mut self.map.get_mut(id).unwrap().0
    }

    pub fn events(&self, id: &Id) -> &Events<T> {
        &self.map.get(&id.id).unwrap().0
    }

    pub fn events_mut(&mut self, id: &Id) -> &mut Events<T> {
        &mut self.map.get_mut(&id.id).unwrap().0
    }

    pub fn update(&mut self) {
        self.map
            .retain(|_, (_, counter)| counter.strong_count() != 0);
        for (_, (events, _)) in self.map.iter_mut() {
            events.update();
        }
    }

    pub fn update_system(mut channels: ResMut<Self>) {
        channels.update();
    }
}
#[derive(SystemParam)]
pub struct ChannelReader<'a, T: Resource> {
    readers: Local<'a, HashMap<usize, (ManualEventReader<T>, Weak<()>)>>,
    channels: Res<'a, Channels<T>>,
}

impl<'a, T: Resource> ChannelReader<'a, T> {
    pub fn read(&mut self, id: &Id) -> impl DoubleEndedIterator<Item = &T> {
        self.readers
            .entry(id.id)
            .or_insert_with(|| (Default::default(), Arc::downgrade(&id.ref_counter)))
            .0
            .iter(&self.channels.events_usize(&id.id))
    }

    pub fn open(&self) -> Id {
        self.channels.reserve()
    }

    pub fn maintain(&mut self) {
        self.readers
            .retain(|_, (_, counter)| counter.strong_count() != 0);
    }
}

#[derive(SystemParam)]
pub struct ChannelWriter<'a, T: Resource> {
    inner: &'a mut ChannelWriterInner<T>,
    channels: Res<'a, Channels<T>>,
}

struct ChannelWriterInner<T: Resource> {
    buffers: HashMap<usize, (Vec<T>, Weak<()>)>,
}

impl<T: Resource> Default for ChannelWriterInner<T> {
    fn default() -> Self {
        Self {
            buffers: Default::default(),
        }
    }
}

impl<T: Resource> Applyable for ChannelWriterInner<T> {
    fn apply(&mut self, _world: &mut bevy_ecs::World, resources: &mut bevy_ecs::Resources) {
        let mut channels = resources.get_mut::<Channels<T>>().unwrap();

        for (id, (v, _)) in self.buffers.iter_mut() {
            for msg in v.drain(..) {
                channels.events_mut_usize(&id).send(msg);
            }
        }
    }

    fn init(&mut self, _world: &bevy_ecs::World, _resources: &mut bevy_ecs::Resources) {}
}

impl<'a, T: Resource> ChannelWriter<'a, T> {
    pub fn send(&mut self, id: &Id, msg: T) {
        self.inner
            .buffers
            .entry(id.id)
            .or_insert_with(|| (Default::default(), Arc::downgrade(&id.ref_counter)))
            .0
            .push(msg);
    }

    pub fn maintain(&mut self) {
        self.inner
            .buffers
            .retain(|_, (_, counter)| counter.strong_count() != 0);
    }

    pub fn open(&self) -> Id {
        self.channels.reserve()
    }
}

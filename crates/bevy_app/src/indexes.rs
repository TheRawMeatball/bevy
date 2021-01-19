use std::hash::Hash;

use bevy_ecs::{Changed, Entity, Query, ResMut};
use bevy_utils::{HashMap, HashSet};

pub struct Index<T: Eq + Hash + Send + Sync + Clone + 'static> {
    forward: HashMap<T, HashSet<Entity>>,
    backward: HashMap<Entity, T>,
}

impl<T: Eq + Hash + Send + Sync + Clone + 'static> Default for Index<T> {
    fn default() -> Self {
        Self {
            forward: Default::default(),
            backward: Default::default(),
        }
    }
}

impl<T: Eq + Hash + Send + Sync + Clone + 'static> Index<T> {
    pub fn get(&self, val: T) -> Option<impl Iterator<Item = &Entity>> {
        Some(self.forward.get(&val)?.iter())
    }

    pub(crate) fn maintain_index(
        mut index: ResMut<Index<T>>,
        change_query: Query<(Entity, &T), Changed<T>>,
        removal_query: Query<Entity>,
    ) {
        for (e, c) in change_query.iter() {
            index.forward.entry(c.clone()).or_default().insert(e);
            index.backward.insert(e, c.clone());
        }

        for e in removal_query.removed::<T>().iter() {
            let removed_component = index.backward.remove(e).unwrap();
            let set = index.forward.get_mut(&removed_component).unwrap();
            set.remove(e);
            if set.is_empty() {
                index.forward.remove(&removed_component);
            }
        }
    }
}

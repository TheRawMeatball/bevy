use bevy_utils::HashSet;
use fixedbitset::FixedBitSet;
use std::{any::TypeId, boxed::Box, hash::Hash, vec::Vec};

use super::{Archetype, World};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum ArchetypeAccess {
    None,
    Read,
    Write,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArchetypeComponent {
    pub archetype_index: u32,
    pub component: TypeId,
}

impl ArchetypeComponent {
    #[inline]
    pub fn new<T: 'static>(archetype_index: u32) -> Self {
        ArchetypeComponent {
            archetype_index,
            component: TypeId::of::<T>(),
        }
    }

    #[inline]
    pub fn new_ty(archetype_index: u32, component: TypeId) -> Self {
        ArchetypeComponent {
            archetype_index,
            component,
        }
    }
}

pub enum QueryAccess {
    None,
    Read(TypeId, &'static str),
    Write(TypeId, &'static str),
    Optional(Box<QueryAccess>),
    With(TypeId, Box<QueryAccess>),
    Without(TypeId, Box<QueryAccess>),
    Union(Vec<QueryAccess>),
}

impl QueryAccess {
    pub fn read<T: 'static>() -> QueryAccess {
        QueryAccess::Read(TypeId::of::<T>(), std::any::type_name::<T>())
    }

    pub fn write<T: 'static>() -> QueryAccess {
        QueryAccess::Write(TypeId::of::<T>(), std::any::type_name::<T>())
    }

    pub fn with<T: 'static>(access: QueryAccess) -> QueryAccess {
        QueryAccess::With(TypeId::of::<T>(), Box::new(access))
    }

    pub fn without<T: 'static>(access: QueryAccess) -> QueryAccess {
        QueryAccess::Without(TypeId::of::<T>(), Box::new(access))
    }

    pub fn optional(access: QueryAccess) -> QueryAccess {
        QueryAccess::Optional(Box::new(access))
    }

    pub fn union(accesses: Vec<QueryAccess>) -> QueryAccess {
        QueryAccess::Union(accesses)
    }

    pub fn get_world_archetype_access(
        &self,
        world: &World,
        mut type_access: Option<&mut TypeAccess<ArchetypeComponent>>,
    ) {
        let archetypes = world.archetypes();
        for (i, archetype) in archetypes.enumerate() {
            let type_access = type_access.as_deref_mut();
            let _ = self.get_access(archetype, i as u32, type_access);
        }
    }

    pub fn get_component_access(&self, type_access: &mut TypeAccess<TypeId>) {
        match self {
            QueryAccess::None => {}
            QueryAccess::Read(ty, _) => type_access.add_read(*ty),
            QueryAccess::Write(ty, _) => type_access.add_write(*ty),
            QueryAccess::Optional(access) => access.get_component_access(type_access),
            QueryAccess::With(_, access) => access.get_component_access(type_access),
            QueryAccess::Without(_, access) => access.get_component_access(type_access),
            QueryAccess::Union(accesses) => {
                for access in accesses {
                    access.get_component_access(type_access);
                }
            }
        }
    }

    pub fn get_type_name(&self, type_id: TypeId) -> Option<&'static str> {
        match self {
            QueryAccess::None => None,
            QueryAccess::Read(current_type_id, name) => {
                if type_id == *current_type_id {
                    Some(*name)
                } else {
                    None
                }
            }
            QueryAccess::Write(current_type_id, name) => {
                if type_id == *current_type_id {
                    Some(*name)
                } else {
                    None
                }
            }
            QueryAccess::Optional(query_access) => query_access.get_type_name(type_id),
            QueryAccess::With(_, query_access) => query_access.get_type_name(type_id),
            QueryAccess::Without(_, query_access) => query_access.get_type_name(type_id),
            QueryAccess::Union(query_accesses) => {
                for query_access in query_accesses.iter() {
                    if let Some(name) = query_access.get_type_name(type_id) {
                        return Some(name);
                    }
                }

                None
            }
        }
    }

    /// Returns how this [QueryAccess] accesses the given `archetype`.
    /// If `type_access` is set, it will populate type access with the types this query reads/writes
    fn get_access(
        &self,
        archetype: &Archetype,
        archetype_index: u32,
        type_access: Option<&mut TypeAccess<ArchetypeComponent>>,
    ) -> Option<ArchetypeAccess> {
        match self {
            QueryAccess::None => Some(ArchetypeAccess::None),
            QueryAccess::Read(ty, _) => {
                if archetype.has_type(*ty) {
                    if let Some(type_access) = type_access {
                        type_access.add_read(ArchetypeComponent::new_ty(archetype_index, *ty));
                    }
                    Some(ArchetypeAccess::Read)
                } else {
                    None
                }
            }
            QueryAccess::Write(ty, _) => {
                if archetype.has_type(*ty) {
                    if let Some(type_access) = type_access {
                        type_access.add_write(ArchetypeComponent::new_ty(archetype_index, *ty));
                    }
                    Some(ArchetypeAccess::Write)
                } else {
                    None
                }
            }
            QueryAccess::Optional(query_access) => {
                if let Some(access) = query_access.get_access(archetype, archetype_index, None) {
                    // only re-run get_archetype_access if we need to set type_access
                    if type_access.is_some() {
                        query_access.get_access(archetype, archetype_index, type_access)
                    } else {
                        Some(access)
                    }
                } else {
                    Some(ArchetypeAccess::Read)
                }
            }
            QueryAccess::With(ty, query_access) => {
                if archetype.has_type(*ty) {
                    query_access.get_access(archetype, archetype_index, type_access)
                } else {
                    None
                }
            }
            QueryAccess::Without(ty, query_access) => {
                if !archetype.has_type(*ty) {
                    query_access.get_access(archetype, archetype_index, type_access)
                } else {
                    None
                }
            }
            QueryAccess::Union(query_accesses) => {
                let mut result = None;
                for query_access in query_accesses {
                    if let Some(access) = query_access.get_access(archetype, archetype_index, None)
                    {
                        result = Some(result.unwrap_or(ArchetypeAccess::Read).max(access));
                    } else {
                        return None;
                    }
                }

                // only set the type access if there is a full match
                if let Some(type_access) = type_access {
                    if result.is_some() {
                        for query_access in query_accesses {
                            query_access.get_access(archetype, archetype_index, Some(type_access));
                        }
                    }
                }

                result
            }
        }
    }
}

/// Provides information about the types a [System] reads and writes
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TypeAccess<T: Hash + Eq + PartialEq> {
    reads_all: bool,
    reads_and_writes: HashSet<T>,
    writes: HashSet<T>,
}

impl<T: Hash + Eq + PartialEq> Default for TypeAccess<T> {
    fn default() -> Self {
        Self {
            reads_all: false,
            reads_and_writes: Default::default(),
            writes: Default::default(),
        }
    }
}

impl<T: Hash + Eq + PartialEq + Copy> TypeAccess<T> {
    pub fn new(reads: Vec<T>, writes: Vec<T>) -> Self {
        let mut type_access = TypeAccess::default();
        for write in writes {
            type_access.add_write(write);
        }
        for read in reads {
            type_access.add_read(read);
        }
        type_access
    }

    pub fn is_compatible(&self, other: &TypeAccess<T>) -> bool {
        if self.reads_all {
            other.writes.is_empty()
        } else if other.reads_all {
            self.writes.is_empty()
        } else {
            self.writes.is_disjoint(&other.reads_and_writes)
                && self.reads_and_writes.is_disjoint(&other.writes)
        }
    }

    pub fn get_conflict<'a>(&'a self, other: &'a TypeAccess<T>) -> Option<&'a T> {
        if self.reads_all {
            other.writes.iter().next()
        } else if other.reads_all {
            self.writes.iter().next()
        } else {
            match self.writes.intersection(&other.reads_and_writes).next() {
                Some(element) => Some(element),
                None => other.writes.intersection(&self.reads_and_writes).next(),
            }
        }
    }

    pub fn extend(&mut self, other: &TypeAccess<T>) {
        self.reads_all = self.reads_all || other.reads_all;
        self.writes.extend(&other.writes);
        self.reads_and_writes.extend(&other.reads_and_writes);
    }

    pub fn add_read(&mut self, ty: T) {
        self.reads_and_writes.insert(ty);
    }

    pub fn add_write(&mut self, ty: T) {
        self.reads_and_writes.insert(ty);
        self.writes.insert(ty);
    }

    pub fn read_all(&mut self) {
        self.reads_all = true;
    }

    pub fn clear(&mut self) {
        self.reads_all = false;
        self.reads_and_writes.clear();
        self.writes.clear();
    }

    pub fn is_read_or_write(&self, ty: &T) -> bool {
        self.reads_all || self.reads_and_writes.contains(ty)
    }

    pub fn is_write(&self, ty: &T) -> bool {
        self.writes.contains(ty)
    }

    pub fn reads_all(&self) -> bool {
        self.reads_all
    }

    /// Returns an iterator of distinct accessed types if only some types are accessed.
    pub fn all_distinct_types(&self) -> Option<impl Iterator<Item = &T>> {
        if !self.reads_all {
            return Some(self.reads_and_writes.iter());
        }
        None
    }

    pub fn condense(&self, all_types: &[T]) -> CondensedTypeAccess {
        if self.reads_all {
            let mut writes = FixedBitSet::with_capacity(all_types.len());
            for (index, access_type) in all_types.iter().enumerate() {
                if self.writes.contains(access_type) {
                    writes.insert(index);
                }
            }
            CondensedTypeAccess {
                reads_all: true,
                reads_and_writes: Default::default(),
                writes,
            }
        } else {
            let mut reads_and_writes = FixedBitSet::with_capacity(all_types.len());
            let mut writes = FixedBitSet::with_capacity(all_types.len());
            for (index, access_type) in all_types.iter().enumerate() {
                if self.writes.contains(access_type) {
                    reads_and_writes.insert(index);
                    writes.insert(index);
                } else if self.reads_and_writes.contains(access_type) {
                    reads_and_writes.insert(index);
                }
            }
            CondensedTypeAccess {
                reads_all: false,
                reads_and_writes,
                writes,
            }
        }
    }
}

// TODO consider making it typed, to enable compiler helping with bug hunting?
#[derive(Default, Debug, Eq, PartialEq, Clone)]
pub struct CondensedTypeAccess {
    reads_all: bool,
    reads_and_writes: FixedBitSet,
    writes: FixedBitSet,
}

impl CondensedTypeAccess {
    pub fn grow(&mut self, bits: usize) {
        self.reads_and_writes.grow(bits);
        self.writes.grow(bits);
    }

    pub fn reads_all(&self) -> bool {
        self.reads_all
    }

    pub fn clear(&mut self) {
        self.reads_all = false;
        self.reads_and_writes.clear();
        self.writes.clear();
    }

    pub fn extend(&mut self, other: &CondensedTypeAccess) {
        self.reads_all = self.reads_all || other.reads_all;
        self.reads_and_writes.union_with(&other.reads_and_writes);
        self.writes.union_with(&other.writes);
    }

    pub fn is_compatible(&self, other: &CondensedTypeAccess) -> bool {
        if self.reads_all {
            0 == other.writes.count_ones(..)
        } else if other.reads_all {
            0 == self.writes.count_ones(..)
        } else {
            self.writes.is_disjoint(&other.reads_and_writes)
                && self.reads_and_writes.is_disjoint(&other.writes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ArchetypeComponent, TypeAccess};
    use crate::{core::World, Entity, Fetch, QueryAccess, WorldQuery};
    use std::vec;

    struct A;
    #[derive(Clone, Eq, PartialEq, Debug)]
    struct B;
    struct C;

    #[test]
    fn query_type_access() {
        let mut world = World::default();
        let e1 = world.spawn((A,));
        let e2 = world.spawn((A, B));
        let e3 = world.spawn((A, B, C));

        let e1_archetype = world.get_entity_location(e1).unwrap().archetype;
        let e2_archetype = world.get_entity_location(e2).unwrap().archetype;
        let e3_archetype = world.get_entity_location(e3).unwrap().archetype;

        let e1_a = ArchetypeComponent::new::<A>(e1_archetype);
        let e2_a = ArchetypeComponent::new::<A>(e2_archetype);
        let e2_b = ArchetypeComponent::new::<B>(e2_archetype);
        let e3_a = ArchetypeComponent::new::<A>(e3_archetype);
        let e3_b = ArchetypeComponent::new::<B>(e3_archetype);
        let e3_c = ArchetypeComponent::new::<C>(e3_archetype);

        let mut a_type_access = TypeAccess::default();
        <(&A,) as WorldQuery>::Fetch::access()
            .get_world_archetype_access(&world, Some(&mut a_type_access));

        assert_eq!(
            a_type_access,
            TypeAccess::new(vec![e1_a, e2_a, e3_a], vec![])
        );

        let mut a_b_type_access = TypeAccess::default();
        <(&A, &B) as WorldQuery>::Fetch::access()
            .get_world_archetype_access(&world, Some(&mut a_b_type_access));

        assert_eq!(
            a_b_type_access,
            TypeAccess::new(vec![e2_a, e2_b, e3_a, e3_b], vec![])
        );

        let mut a_bmut_type_access = TypeAccess::default();
        <(&A, &mut B) as WorldQuery>::Fetch::access()
            .get_world_archetype_access(&world, Some(&mut a_bmut_type_access));

        assert_eq!(
            a_bmut_type_access,
            TypeAccess::new(vec![e2_a, e3_a], vec![e2_b, e3_b])
        );

        let mut a_option_bmut_type_access = TypeAccess::default();
        <(Entity, &A, Option<&mut B>) as WorldQuery>::Fetch::access()
            .get_world_archetype_access(&world, Some(&mut a_option_bmut_type_access));

        assert_eq!(
            a_option_bmut_type_access,
            TypeAccess::new(vec![e1_a, e2_a, e3_a], vec![e2_b, e3_b])
        );

        let mut a_with_b_type_access = TypeAccess::default();
        QueryAccess::with::<B>(<&A as WorldQuery>::Fetch::access())
            .get_world_archetype_access(&world, Some(&mut a_with_b_type_access));

        assert_eq!(
            a_with_b_type_access,
            TypeAccess::new(vec![e2_a, e3_a], vec![])
        );

        let mut a_with_b_option_c_type_access = TypeAccess::default();
        QueryAccess::with::<B>(<(&A, Option<&mut C>) as WorldQuery>::Fetch::access())
            .get_world_archetype_access(&world, Some(&mut a_with_b_option_c_type_access));

        assert_eq!(
            a_with_b_option_c_type_access,
            TypeAccess::new(vec![e2_a, e3_a], vec![e3_c])
        );
    }
}

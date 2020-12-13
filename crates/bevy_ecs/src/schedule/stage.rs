use std::{any::TypeId, borrow::Cow};

use crate::{
    ArchetypeComponent, IntoSystem, Resources, RunCriteria, ShouldRun, System, SystemId,
    TypeAccess, World,
};
use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};

use super::{ParallelSystemStageExecutor, SerialSystemStageExecutor, SystemStageExecutor};

pub enum StageError {
    SystemAlreadyExists(SystemId),
}

pub trait Stage: Downcast + Send + Sync {
    fn run(&mut self, world: &mut World, resources: &mut Resources);
}

impl_downcast!(Stage);

type Label = &'static str; // TODO

#[derive(Clone, Copy)]
pub struct SystemIndex {
    pub set: usize,
    pub system: usize,
}

pub struct SystemStage {
    system_sets: Vec<SystemSet>,
    labels: HashMap<Label, SystemIndex>,
    executor: Box<dyn SystemStageExecutor>,
    run_criteria: RunCriteria,
}

impl SystemStage {
    pub fn new(executor: Box<dyn SystemStageExecutor>) -> Self {
        SystemStage {
            executor,
            labels: Default::default(),
            run_criteria: Default::default(),
            system_sets: vec![SystemSet::default()],
        }
    }

    pub fn single<Params, S: System<In = (), Out = ()>, Into: IntoSystem<Params, S>>(
        system: Into,
    ) -> Self {
        Self::serial().with_system(system)
    }

    pub fn serial() -> Self {
        Self::new(Box::new(SerialSystemStageExecutor::default()))
    }

    pub fn parallel() -> Self {
        Self::new(Box::new(ParallelSystemStageExecutor::default()))
    }

    pub fn with_system<S, Params, IntoS>(mut self, system: IntoS) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system(system);
        self
    }

    pub fn with_system_labeled<S, Params, IntoS>(mut self, system: IntoS, label: Label) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_labeled(system, label);
        self
    }

    pub fn with_system_with_dependencies<S, Params, IntoS>(
        mut self,
        system: IntoS,
        dependencies: &[Label],
    ) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_with_dependencies(system, dependencies);
        self
    }

    pub fn with_system_labeled_with_dependencies<S, Params, IntoS>(
        mut self,
        system: IntoS,
        label: Label,
        dependencies: &[Label],
    ) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_labeled_with_dependencies(system, label, dependencies);
        self
    }

    pub fn with_system_set(mut self, system_set: SystemSet) -> Self {
        self.add_system_set(system_set);
        self
    }

    pub fn with_run_criteria<S, Params, IntoS>(mut self, system: IntoS) -> Self
    where
        S: System<In = (), Out = ShouldRun>,
        IntoS: IntoSystem<Params, S>,
    {
        self.run_criteria.set(Box::new(system.system()));
        self
    }

    pub fn add_system_set(&mut self, system_set: SystemSet) -> &mut Self {
        self.system_sets.push(system_set);
        self
    }

    pub fn add_system<S, Params, IntoS>(&mut self, system: IntoS) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.system_sets[0].add_system(system);
        self
    }

    pub fn add_system_labeled<S, Params, IntoS>(&mut self, system: IntoS, label: Label) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.system_sets[0].add_system_labeled(system, label);
        self
    }

    pub fn add_system_with_dependencies<S, Params, IntoS>(
        &mut self,
        system: IntoS,
        dependencies: &[Label],
    ) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.system_sets[0].add_system_with_dependencies(system, dependencies);
        self
    }

    pub fn add_system_labeled_with_dependencies<S, Params, IntoS>(
        &mut self,
        system: IntoS,
        label: Label,
        dependencies: &[Label],
    ) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.system_sets[0].add_system_labeled_with_dependencies(system, label, dependencies);
        self
    }

    pub fn add_system_boxed(&mut self, system: Box<dyn System<In = (), Out = ()>>) -> &mut Self {
        self.system_sets[0].add_system_boxed(system);
        self
    }

    pub fn add_system_boxed_labeled(
        &mut self,
        system: Box<dyn System<In = (), Out = ()>>,
        label: Label,
    ) -> &mut Self {
        self.system_sets[0].add_system_boxed_labeled(system, label);
        self
    }

    pub fn add_system_boxed_with_dependencies(
        &mut self,
        system: Box<dyn System<In = (), Out = ()>>,
        dependencies: &[Label],
    ) -> &mut Self {
        self.system_sets[0].add_system_boxed_with_dependencies(system, dependencies);
        self
    }

    pub fn add_system_boxed_labeled_with_dependencies(
        &mut self,
        system: Box<dyn System<In = (), Out = ()>>,
        label: Label,
        dependencies: &[Label],
    ) -> &mut Self {
        self.system_sets[0].add_system_boxed_labeled_with_dependencies(system, label, dependencies);
        self
    }

    pub fn get_executor<T: SystemStageExecutor>(&self) -> Option<&T> {
        self.executor.downcast_ref()
    }

    pub fn get_executor_mut<T: SystemStageExecutor>(&mut self) -> Option<&mut T> {
        self.executor.downcast_mut()
    }

    pub fn run_once(&mut self, world: &mut World, resources: &mut Resources) {
        for (set_index, system_set) in self.system_sets.iter_mut().enumerate() {
            for &system_index in system_set.changed_systems() {
                if let Some(label) = system_set.system_label(system_index) {
                    self.labels.insert(
                        label,
                        SystemIndex {
                            set: set_index,
                            system: system_index,
                        },
                    );
                }
            }
            system_set.for_each_changed_system(|system| system.initialize(world, resources));
        }
        self.executor
            .execute_stage(&mut self.system_sets, &self.labels, world, resources);
        for system_set in self.system_sets.iter_mut() {
            system_set.clear_changed_systems();
        }
    }
}

impl Stage for SystemStage {
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        loop {
            match self.run_criteria.should_run(world, resources) {
                ShouldRun::No => return,
                ShouldRun::Yes => {
                    self.run_once(world, resources);
                    return;
                }
                ShouldRun::YesAndLoop => {
                    self.run_once(world, resources);
                }
                ShouldRun::NoAndLoop => {
                    panic!("`NoAndLoop` run criteria would loop infinitely in this situation.")
                }
            }
        }
    }
}

#[derive(Default)]
pub struct SystemSet {
    systems: Vec<Box<dyn System<In = (), Out = ()>>>,
    labels: Vec<Option<Label>>,
    dependencies: Vec<Vec<Label>>,
    run_criteria: RunCriteria,
    changed_systems: Vec<usize>,
}

impl SystemSet {
    // TODO: ideally this returns an iterator, but impl Iterator can't be used in this context (yet) and a custom iterator isn't worth it
    pub fn for_each_changed_system(
        &mut self,
        mut func: impl FnMut(&mut dyn System<In = (), Out = ()>),
    ) {
        for index in self.changed_systems.iter_mut() {
            func(&mut *self.systems[*index])
        }
    }

    pub fn changed_systems(&self) -> &[usize] {
        &self.changed_systems
    }

    pub fn clear_changed_systems(&mut self) {
        self.changed_systems.clear()
    }

    pub(crate) fn run_criteria(&self) -> &RunCriteria {
        &self.run_criteria
    }

    pub(crate) fn run_criteria_mut(&mut self) -> &mut RunCriteria {
        &mut self.run_criteria
    }

    pub fn systems(&self) -> &[Box<dyn System<In = (), Out = ()>>] {
        &self.systems
    }

    pub fn systems_mut(&mut self) -> &mut [Box<dyn System<In = (), Out = ()>>] {
        &mut self.systems
    }

    pub fn system_label(&self, index: usize) -> Option<Label> {
        self.labels[index]
    }

    pub fn system_dependencies(&self, index: usize) -> &[Label] {
        &self.dependencies[index]
    }

    pub fn with_system<S, Params, IntoS>(mut self, system: IntoS) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system(system.system());
        self
    }

    pub fn with_system_labeled<S, Params, IntoS>(mut self, system: IntoS, label: Label) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_labeled(system.system(), label);
        self
    }

    pub fn with_system_with_dependencies<S, Params, IntoS>(
        mut self,
        system: IntoS,
        dependencies: &[Label],
    ) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_with_dependencies(system.system(), dependencies);
        self
    }

    pub fn with_system_labeled_with_dependencies<S, Params, IntoS>(
        mut self,
        system: IntoS,
        label: Label,
        dependencies: &[Label],
    ) -> Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_labeled_with_dependencies(system.system(), label, dependencies);
        self
    }

    pub fn add_system<S, Params, IntoS>(&mut self, system: IntoS) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_boxed(Box::new(system.system()))
    }

    pub fn add_system_labeled<S, Params, IntoS>(&mut self, system: IntoS, label: Label) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_boxed_labeled(Box::new(system.system()), label)
    }

    pub fn add_system_with_dependencies<S, Params, IntoS>(
        &mut self,
        system: IntoS,
        dependencies: &[Label],
    ) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_boxed_with_dependencies(Box::new(system.system()), dependencies)
    }

    pub fn add_system_labeled_with_dependencies<S, Params, IntoS>(
        &mut self,
        system: IntoS,
        label: Label,
        dependencies: &[Label],
    ) -> &mut Self
    where
        S: System<In = (), Out = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_boxed_labeled_with_dependencies(
            Box::new(system.system()),
            label,
            dependencies,
        )
    }

    pub fn add_system_boxed(&mut self, system: Box<dyn System<In = (), Out = ()>>) -> &mut Self {
        self.add_system_boxed_with_dependencies(system, &[])
    }

    pub fn add_system_boxed_labeled(
        &mut self,
        system: Box<dyn System<In = (), Out = ()>>,
        label: Label,
    ) -> &mut Self {
        self.add_system_boxed_labeled_with_dependencies(system, label, &[])
    }

    pub fn add_system_boxed_with_dependencies(
        &mut self,
        system: Box<dyn System<In = (), Out = ()>>,
        dependencies: &[Label],
    ) -> &mut Self {
        self.systems.push(system);
        self.changed_systems.push(self.systems.len());
        self.labels.push(None);
        self.dependencies.push(dependencies.to_vec());
        self
    }

    pub fn add_system_boxed_labeled_with_dependencies(
        &mut self,
        system: Box<dyn System<In = (), Out = ()>>,
        label: Label,
        dependencies: &[Label],
    ) -> &mut Self {
        self.systems.push(system);
        self.changed_systems.push(self.systems.len());
        self.labels.push(Some(label));
        self.dependencies.push(dependencies.to_vec());
        self
    }
}

impl<S: System<In = (), Out = ()>> From<S> for SystemStage {
    fn from(system: S) -> Self {
        SystemStage::single(system)
    }
}

pub trait IntoStage<Params> {
    type Stage: Stage;
    fn into_stage(self) -> Self::Stage;
}

impl<Params, S: System<In = (), Out = ()>, IntoS: IntoSystem<Params, S>> IntoStage<(Params, S)>
    for IntoS
{
    type Stage = SystemStage;

    fn into_stage(self) -> Self::Stage {
        SystemStage::single(self)
    }
}

impl<S: Stage> IntoStage<()> for S {
    type Stage = S;

    fn into_stage(self) -> Self::Stage {
        self
    }
}

pub struct RunOnce {
    ran: bool,
    system_id: SystemId,
    archetype_component_access: TypeAccess<ArchetypeComponent>,
    resource_access: TypeAccess<TypeId>,
}

impl Default for RunOnce {
    fn default() -> Self {
        Self {
            ran: false,
            system_id: SystemId::new(),
            archetype_component_access: Default::default(),
            resource_access: Default::default(),
        }
    }
}

impl System for RunOnce {
    type In = ();
    type Out = ShouldRun;

    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<RunOnce>())
    }

    fn id(&self) -> SystemId {
        self.system_id
    }

    fn update_access(&mut self, _world: &World) {}

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn is_thread_local(&self) -> bool {
        false
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: Self::In,
        _world: &World,
        _resources: &Resources,
    ) -> Option<Self::Out> {
        Some(if self.ran {
            ShouldRun::No
        } else {
            self.ran = true;
            ShouldRun::Yes
        })
    }

    fn run_exclusive(&mut self, _world: &mut World, _resources: &mut Resources) {}

    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}

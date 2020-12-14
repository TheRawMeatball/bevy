#![allow(dead_code, unused_variables, unused_imports)]

use std::ops::Range;

use async_channel::{Receiver, Sender};
use bevy_tasks::{ComputeTaskPool, Scope, TaskPool};
use bevy_utils::{tracing::trace, HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;

use crate::{ArchetypesGeneration, Resources, System, SystemIndex, SystemSet, TypeAccess, World};

type Label = &'static str; // TODO

pub trait SystemStageExecutor: Downcast + Send + Sync {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        system_labels: &HashMap<Label, SystemIndex>,
        world: &mut World,
        resources: &mut Resources,
    );
}

impl_downcast!(SystemStageExecutor);

pub struct SerialSystemStageExecutor {
    /// Determines if a system has had its exclusive part already executed.
    exclusive_ran: FixedBitSet,
    last_archetypes_generation: ArchetypesGeneration,
}

impl Default for SerialSystemStageExecutor {
    fn default() -> Self {
        Self {
            exclusive_ran: FixedBitSet::with_capacity(64),
            // MAX ensures metadata will be initialized on first run.
            last_archetypes_generation: ArchetypesGeneration(u64::MAX),
        }
    }
}

impl SystemStageExecutor for SerialSystemStageExecutor {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        system_labels: &HashMap<Label, SystemIndex>,
        world: &mut World,
        resources: &mut Resources,
    ) {
        self.exclusive_ran.clear();
        let mut index = 0;
        for system_set in system_sets.iter_mut() {
            self.exclusive_ran.grow(index + system_set.systems_len());
            for system_index in 0..system_set.systems_len() {
                // TODO handle order of operations set by dependencies.
                let is_exclusive = {
                    let system = &system_set.system(system_index);
                    system.archetype_component_access().writes_all()
                        || system.resource_access().writes_all()
                };
                if is_exclusive {
                    system_set
                        .system_mut(system_index)
                        .run_exclusive(world, resources);
                    self.exclusive_ran.set(index, true);
                }
                index += 1;
            }
        }
        if self.last_archetypes_generation != world.archetypes_generation() {
            for system_set in system_sets.iter_mut() {
                for system in system_set.systems_mut() {
                    system.update_access(world);
                    system.run((), world, resources);
                }
            }
            self.last_archetypes_generation = world.archetypes_generation();
        } else {
            for system_set in system_sets.iter_mut() {
                system_set.for_each_changed_system(|system| system.update_access(world));
                for system in system_set.systems_mut() {
                    system.run((), world, resources);
                }
            }
        }
        let mut index = 0;
        for system_set in system_sets.iter_mut() {
            for system in system_set.systems_mut() {
                if !self.exclusive_ran[index] {
                    system.run_exclusive(world, resources);
                }
                index += 1;
            }
        }
    }
}

struct ParallelSystemSchedulingData {
    /// System's index in the system sets.
    index: SystemIndex,
    // TODO ditch? Rename?
    /// Ensures a system can be accessed unsafely only once a frame.
    was_accessed_unsafely: bool,
    /// Used to signal the system's task to start the system.
    start_sender: Sender<()>,
    /// Receives the signal to start the system.
    start_receiver: Receiver<()>,
    /// Indices of systems that depend on this one, used to decrement their
    /// dependency counters when this system finishes.
    dependants: Vec<usize>,
    /// Total amount of dependencies this system has.
    dependencies_total: usize,
    /// Amount of unsatisfied dependencies, when it reaches 0 the system is queued to be started.
    dependencies_now: usize,
}

pub struct ParallelSystemStageExecutor {
    /// When archetypes change a counter is bumped - we cache the state of that counter when it was
    /// last read here so that we can detect when archetypes are changed
    last_archetypes_generation: ArchetypesGeneration,
    /// Systems with exclusive access that run before parallel systems.
    on_start_exclusives: Vec<SystemIndex>,
    /// Systems with exclusive access that run after parallel systems.
    on_end_exclusives: Vec<SystemIndex>,
    /// Systems that run in parallel.
    parallel: Vec<ParallelSystemSchedulingData>,
    /// Used by systems to notify the executor that they have finished.
    finish_sender: Sender<usize>,
    /// Receives finish events from systems.
    finish_receiver: Receiver<usize>,
    /// Parallel systems that must run on the main thread.
    thread_local: FixedBitSet,
    /// Parallel systems that should be started at next opportunity.
    queued: FixedBitSet,
    /// Parallel systems that are currently running.
    running: FixedBitSet,
    /// Scratch space to avoid reallocating a vector when updating dependency counters.
    dependants_scratch: Vec<usize>,
}

impl Default for ParallelSystemStageExecutor {
    fn default() -> Self {
        let (finish_sender, finish_receiver) = async_channel::unbounded();
        Self {
            // MAX ensures metadata will be initialized on first run.
            last_archetypes_generation: ArchetypesGeneration(u64::MAX),
            on_start_exclusives: Default::default(),
            on_end_exclusives: Default::default(),
            parallel: Default::default(),
            finish_sender,
            finish_receiver,
            thread_local: Default::default(),
            queued: Default::default(),
            running: Default::default(),
            dependants_scratch: Default::default(),
        }
    }
}

impl SystemStageExecutor for ParallelSystemStageExecutor {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        system_labels: &HashMap<Label, SystemIndex>,
        world: &mut World,
        resources: &mut Resources,
    ) {
        // TODO run criteria

        // Cache dependencies for populating systems' dependants.
        let mut all_dependencies = Vec::new();
        for system_set in system_sets {
            for system in system_set.systems() {
                // TODO all of this. Split to .prepare() too
            }
        }

        self.thread_local.grow(self.parallel.len());
        self.queued.grow(self.parallel.len());
        self.running.grow(self.parallel.len());

        for index in &self.on_start_exclusives {
            system_sets[index.set]
                .system_mut(index.system)
                .run_exclusive(world, resources);
        }

        let compute_pool = resources
            .get_or_insert_with(|| ComputeTaskPool(TaskPool::default()))
            .clone();
        compute_pool.scope(|scope| {
            // Spawn tasks for thread-agnostic systems.
            self.spawn_system_tasks(scope, system_sets, world, resources);
            // All systems have been ran if there are no queued or running systems.
            while 0 < self.queued.count_ones(..) + self.running.count_ones(..) {
                // Try running a thread-local system on the main thread.
                self.run_thread_local(system_sets, world, resources);
                // Try running thread-agnostic systems.
                compute_pool.scope(|scope| {
                    scope.spawn(async {
                        self.start_runnable_queued(system_sets, world, resources)
                            .await;
                        // Avoids deadlocking if there's nothing to wait for.
                        if 0 < self.running.count_ones(..) {
                            self.process_finished(system_sets, world, resources).await;
                        }
                    })
                });
            }
        });

        // TODO do we want this before or after the exclusives? Do we update access between?
        for scheduling_data in &self.parallel {
            let index = scheduling_data.index;
            system_sets[index.set]
                .system_mut(index.system)
                .run_exclusive(world, resources);
        }
        for index in &self.on_end_exclusives {
            system_sets[index.set]
                .system_mut(index.system)
                .run_exclusive(world, resources);
        }
    }
}

impl ParallelSystemStageExecutor {
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_system_mut_unsafe<'a>(
        &mut self,
        index: usize,
        system_sets: &'a [SystemSet],
    ) -> &'a mut dyn System<In = (), Out = ()> {
        let was_accessed_unsafely = &mut self.parallel[index].was_accessed_unsafely;
        assert!(!*was_accessed_unsafely);
        *was_accessed_unsafely = true;
        let index = self.parallel[index].index;
        system_sets[index.set].system_mut_unsafe(index.system)
    }

    fn can_start_now(&self, index: usize) -> bool {
        let system = &self.parallel[index];
        for other in self.queued.ones().map(|index| &self.parallel[index]) {

        }
        true
    }

    fn spawn_system_tasks<'scope>(
        &mut self,
        scope: &mut Scope<'scope, ()>,
        system_sets: &'scope [SystemSet],
        world: &'scope World,
        resources: &'scope Resources,
    ) {
        for index in 0..self.parallel.len() {
            if !self.thread_local[index] {
                let start_receiver = self.parallel[index].start_receiver.clone();
                let finish_sender = self.finish_sender.clone();
                let system = unsafe { self.get_system_mut_unsafe(index, system_sets) };
                scope.spawn(async move {
                    start_receiver
                        .recv()
                        .await
                        .unwrap_or_else(|error| unreachable!(error));
                    unsafe { system.run_unsafe((), world, resources) };
                    finish_sender
                        .send(index)
                        .await
                        .unwrap_or_else(|error| unreachable!(error));
                });
            }
        }
    }

    fn run_thread_local(
        &mut self,
        system_sets: &[SystemSet],
        world: &World,
        resources: &Resources,
    ) {
        for index in self.queued.intersection(&self.thread_local) {
            if self.can_start_now(index) {
                unsafe {
                    self.get_system_mut_unsafe(index, system_sets)
                        .run_unsafe((), world, resources);
                }
                self.queued.set(index, false);
                // Decrement dependency counters, queue systems that had their
                // dependencies satisfied.
                self.dependants_scratch
                    .extend(&self.parallel[index].dependants);
                for index in self.dependants_scratch.drain(..) {
                    let dependent = &mut self.parallel[index];
                    dependent.dependencies_now -= 1;
                    if dependent.dependencies_now == 0 {
                        self.queued.insert(index);
                    }
                }
                break;
            }
        }
    }

    async fn start_runnable_queued(
        &mut self,
        system_sets: &[SystemSet],
        world: &World,
        resources: &Resources,
    ) {
        for index in self.queued.difference(&self.thread_local) {
            if self.can_start_now(index) {
                self.parallel[index]
                    .start_sender
                    .send(())
                    .await
                    .unwrap_or_else(|error| unreachable!(error));
                self.running.set(index, true);
            }
        }
        self.queued.difference_with(&self.running);
    }

    async fn process_finished(
        &mut self,
        system_sets: &[SystemSet],
        world: &World,
        resources: &Resources,
    ) {
        // Wait until at least one system has finished.
        let index = self
            .finish_receiver
            .recv()
            .await
            .unwrap_or_else(|error| unreachable!(error));
        self.running.set(index, false);
        self.dependants_scratch
            .extend(&self.parallel[index].dependants);
        // Process other systems than may have finished.
        while let Ok(index) = self.finish_receiver.try_recv() {
            self.running.set(index, false);
            self.dependants_scratch
                .extend(&self.parallel[index].dependants);
        }
        // Decrement dependency counters, queue systems that had their
        // dependencies satisfied.
        for index in self.dependants_scratch.drain(..) {
            let dependent = &mut self.parallel[index];
            dependent.dependencies_now -= 1;
            if dependent.dependencies_now == 0 {
                self.queued.insert(index);
            }
        }
    }
}

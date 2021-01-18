use async_channel::{Receiver, Sender};
use bevy_tasks::{ComputeTaskPool, Scope, TaskPool};
use bevy_utils::{HashMap, HashSet};
use fixedbitset::FixedBitSet;

use super::ExecutorCommonMethods;
use crate::{
    ArchetypesGeneration, CondensedTypeAccess, Resources,
    ShouldRun::{self, *},
    System, SystemIndex, SystemSet, SystemStageExecutor, World,
};

struct ParallelSystemSchedulingData {
    /// System's index in the system sets.
    index: SystemIndex,
    // TODO ditch? Rename?
    /// Ensures a system can be accessed unsafely only once per iteration.
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
    /// Archetype-component access information condensed into executor-specific bitsets.
    archetype_component_access: CondensedTypeAccess,
    /// Resource access information condensed into executor-specific bitsets.
    resource_access: CondensedTypeAccess,
}

pub struct ParallelSystemStageExecutor {
    /// Last archetypes generation observed by parallel systems.
    last_archetypes_generation: ArchetypesGeneration,
    /// Cached results of system sets' run criteria evaluation.
    system_set_should_run: Vec<ShouldRun>,
    /// Systems that run in parallel.
    parallel: Vec<ParallelSystemSchedulingData>,
    /// Used by systems to notify the executor that they have finished.
    finish_sender: Sender<usize>,
    /// Receives finish events from systems.
    finish_receiver: Receiver<usize>,
    /// Parallel systems that should run this iteration.
    should_run: FixedBitSet,
    /// Parallel systems that must run on the main thread.
    thread_local: FixedBitSet,
    /// Parallel systems that should be started at next opportunity.
    queued: FixedBitSet,
    /// Parallel systems that are currently running.
    running: FixedBitSet,
    /// Compound archetype-component access information of currently running systems.
    active_archetype_component_access: CondensedTypeAccess,
    /// Compound resource access information of currently running systems.
    active_resource_access: CondensedTypeAccess,
    /// Scratch space to avoid reallocating a vector when updating dependency counters.
    dependants_scratch: Vec<usize>,
}

impl Default for ParallelSystemStageExecutor {
    fn default() -> Self {
        let (finish_sender, finish_receiver) = async_channel::unbounded();
        Self {
            // MAX ensures metadata will be initialized on first run.
            last_archetypes_generation: ArchetypesGeneration(u64::MAX),
            system_set_should_run: Default::default(),
            parallel: Default::default(),
            finish_sender,
            finish_receiver,
            should_run: Default::default(),
            thread_local: Default::default(),
            queued: Default::default(),
            running: Default::default(),
            active_archetype_component_access: Default::default(),
            active_resource_access: Default::default(),
            dependants_scratch: Default::default(),
        }
    }
}

impl ExecutorCommonMethods for ParallelSystemStageExecutor {
    fn system_set_should_run(&self) -> &Vec<ShouldRun> {
        &self.system_set_should_run
    }

    fn system_set_should_run_mut(&mut self) -> &mut Vec<ShouldRun> {
        &mut self.system_set_should_run
    }
}

impl SystemStageExecutor for ParallelSystemStageExecutor {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        at_start: &[SystemIndex],
        before_commands: &[SystemIndex],
        at_end: &[SystemIndex],
        parallel_dependencies: &HashMap<SystemIndex, Vec<SystemIndex>>,
        world: &mut World,
        resources: &mut Resources,
    ) {
        let mut has_work = self.evaluate_run_criteria(system_sets, world, resources);
        if !has_work {
            return;
        }
        if system_sets.iter().any(|system_set| system_set.is_dirty()) {
            self.rebuild_scheduling_data(system_sets, parallel_dependencies, world);
        }
        while has_work {
            // Run systems that want to be at the start of stage.
            self.run_systems_sequence(at_start, system_sets, world, resources);

            if self.last_archetypes_generation != world.archetypes_generation() {
                self.update_parallel_access(system_sets, world);
                self.last_archetypes_generation = world.archetypes_generation();
            }

            // Run parallel systems.
            let compute_pool = resources
                .get_or_insert_with(|| ComputeTaskPool(TaskPool::default()))
                .clone();
            compute_pool.scope(|scope| {
                self.prepare_parallel_systems(scope, system_sets, world, resources);
                // All systems have been ran if there are no queued or running systems.
                while 0 < self.queued.count_ones(..) + self.running.count_ones(..) {
                    // Try running a thread-local system on the main thread.
                    self.run_a_thread_local(system_sets, world, resources);
                    // Try running thread-agnostic systems.
                    compute_pool.scope(|scope| scope.spawn(self.run_thread_agnostic()));
                }
            });

            // Run systems that want to be between parallel systems and their command buffers.
            self.run_systems_sequence(before_commands, system_sets, world, resources);

            // Apply parallel systems' buffers.
            // TODO sort wrt dependencies?
            // TODO rewrite to use the bitset?
            for scheduling_data in &self.parallel {
                let index = scheduling_data.index;
                if let Yes | YesAndLoop = self.system_set_should_run[index.set] {
                    system_sets[index.set]
                        .parallel_system_mut(index.system)
                        .apply_buffers(world, resources);
                }
            }

            // Run systems that want to be at the end of stage.
            self.run_systems_sequence(at_end, system_sets, world, resources);

            has_work = self.reevaluate_run_criteria(system_sets, world, resources);
        }
    }
}

impl ParallelSystemStageExecutor {
    /// Discards and rebuilds parallel system scheduling data and lists of exclusives.
    /// Updates access of parallel systems if needed.
    fn rebuild_scheduling_data(
        &mut self,
        system_sets: &mut [SystemSet],
        parallel_systems_dependencies: &HashMap<SystemIndex, Vec<SystemIndex>>,
        world: &mut World,
    ) {
        self.parallel.clear();
        self.thread_local.clear();

        // Collect all distinct types accessed by parallel systems in order to condense their
        // access sets into bitsets.
        let mut all_archetype_components = HashSet::default();
        let mut all_resource_types = HashSet::default();
        let mut gather_distinct_access_types = |system: &dyn System<In = (), Out = ()>| {
            if let Some(archetype_components) =
                system.archetype_component_access().all_distinct_types()
            {
                all_archetype_components.extend(archetype_components);
            }
            if let Some(resources) = system.resource_access().all_distinct_types() {
                all_resource_types.extend(resources);
            }
        };
        // If the archetypes were changed too, system access should be updated
        // before gathering the types.
        let mut parallel_systems_len = 0;
        if self.last_archetypes_generation != world.archetypes_generation() {
            for system_set in system_sets.iter_mut() {
                parallel_systems_len += system_set.parallel_systems_len();
                for system in system_set.parallel_systems_mut() {
                    system.update_access(world);
                    gather_distinct_access_types(system);
                }
            }
            self.last_archetypes_generation = world.archetypes_generation();
        } else {
            for system_set in system_sets.iter() {
                parallel_systems_len += system_set.parallel_systems_len();
                for system in system_set.parallel_systems() {
                    gather_distinct_access_types(system);
                }
            }
        }
        let all_archetype_components = all_archetype_components.drain().collect::<Vec<_>>();
        let all_resource_types = all_resource_types.drain().collect::<Vec<_>>();

        self.should_run.grow(parallel_systems_len);
        self.thread_local.grow(parallel_systems_len);
        self.queued.grow(parallel_systems_len);
        self.running.grow(parallel_systems_len);

        // Construct scheduling data for parallel systems,
        // cache mapping of parallel system's `SystemIndex` to its index in the list.
        let mut parallel_systems_mapping =
            HashMap::with_capacity_and_hasher(parallel_systems_len, Default::default());
        for (set_index, system_set) in system_sets.iter_mut().enumerate() {
            for (system_index, system) in system_set.parallel_systems().enumerate() {
                let index = SystemIndex {
                    set: set_index,
                    system: system_index,
                };
                parallel_systems_mapping.insert(index, self.parallel.len());
                let dependencies_total = parallel_systems_dependencies
                    .get(&index)
                    .map_or(0, |dependencies| dependencies.len());
                if system.is_thread_local() {
                    self.thread_local.insert(self.parallel.len());
                }
                let (start_sender, start_receiver) = async_channel::bounded(1);
                self.parallel.push(ParallelSystemSchedulingData {
                    index,
                    was_accessed_unsafely: false,
                    start_sender,
                    start_receiver,
                    dependants: vec![],
                    dependencies_total,
                    dependencies_now: 0,
                    archetype_component_access: system
                        .archetype_component_access()
                        .condense(&all_archetype_components),
                    resource_access: system.resource_access().condense(&all_resource_types),
                });
            }
        }
        // Populate the dependants lists in the scheduling data using the mapping.
        for (dependant, dependencies) in parallel_systems_dependencies.iter() {
            let dependant = parallel_systems_mapping[dependant];
            for dependency in dependencies {
                let dependency = parallel_systems_mapping[dependency];
                self.parallel[dependency].dependants.push(dependant);
            }
        }
    }

    /// Updates access and recondenses the archetype component bitsets of parallel systems.
    fn update_parallel_access(&mut self, system_sets: &mut [SystemSet], world: &mut World) {
        let mut all_archetype_components = HashSet::default();
        for scheduling_data in self
            .parallel
            .iter_mut()
            .filter(|data| !data.archetype_component_access.reads_all())
        {
            let system = system_sets[scheduling_data.index.set]
                .parallel_system_mut(scheduling_data.index.system);
            system.update_access(world);
            if let Some(archetype_components) =
                system.archetype_component_access().all_distinct_types()
            {
                all_archetype_components.extend(archetype_components);
            }
        }
        let all_archetype_components = all_archetype_components.drain().collect::<Vec<_>>();
        for scheduling_data in self
            .parallel
            .iter_mut()
            .filter(|data| !data.archetype_component_access.reads_all())
        {
            let system = system_sets[scheduling_data.index.set]
                .parallel_system_mut(scheduling_data.index.system);
            scheduling_data.archetype_component_access = system
                .archetype_component_access()
                .condense(&all_archetype_components);
        }
    }

    /// Gives mutable access to the system at given index.
    /// Can be done only once per system per iteration.
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
        system_sets[index.set].parallel_system_mut_unsafe(index.system)
    }

    /// Determines if the parallel system with given index doesn't conflict already running systems.
    fn can_start_now(&self, index: usize) -> bool {
        let system = &self.parallel[index];
        system
            .resource_access
            .is_compatible(&self.active_resource_access)
            && system
                .archetype_component_access
                .is_compatible(&self.active_archetype_component_access)
    }

    /// Resets safety bits, populates `should_run` bitset,
    /// spawns tasks for thread-agnostic systems, queues systems with no dependencies.
    fn prepare_parallel_systems<'scope>(
        &mut self,
        scope: &mut Scope<'scope, ()>,
        system_sets: &'scope [SystemSet],
        world: &'scope World,
        resources: &'scope Resources,
    ) {
        for index in 0..self.parallel.len() {
            // Reset safety bit.
            self.parallel[index].was_accessed_unsafely = false;
            let should_run = match self.system_set_should_run[self.parallel[index].index.set] {
                Yes | YesAndLoop => true,
                No | NoAndLoop => false,
            };
            // Cache which systems should be ran, to avoid queueing them later.
            self.should_run.set(index, should_run);
            if should_run {
                // Spawn tasks for thread-agnostic systems.
                if !self.thread_local[index] {
                    self.spawn_system_task(index, scope, system_sets, world, resources);
                }
                // Queue systems with no dependencies, reset dependency counters.
                let system_data = &mut self.parallel[index];
                if system_data.dependencies_total == 0 {
                    self.queued.insert(index);
                } else {
                    system_data.dependencies_now = system_data.dependencies_total;
                }
            }
        }
    }

    /// Spawns the task for parallel system with given index. Trips the safety bit.
    /// Will likely lead to a panic when used with a thread-local system.
    fn spawn_system_task<'scope>(
        &mut self,
        index: usize,
        scope: &mut Scope<'scope, ()>,
        system_sets: &'scope [SystemSet],
        world: &'scope World,
        resources: &'scope Resources,
    ) {
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

    /// Tries to run one non-conflicting queued thread-local system on the main thread, decrements
    /// dependency counters for its dependants, enqueues dependants with all satisfied dependencies
    /// if they should be ran this iteration. Trips the safety bit.
    fn run_a_thread_local(
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
                self.dependants_scratch
                    .extend(&self.parallel[index].dependants);
                self.update_counters_and_queue();
                break;
            }
        }
    }

    /// Starts all non-conflicting queued thread-agnostic systems,
    /// moves them from `queued` to `running`. If there are running systems, waits for one or more
    /// of them to finish, decrements dependency counters of their dependants,
    /// enqueues dependants with all satisfied dependencies if they should be ran this iteration.
    async fn run_thread_agnostic(&mut self) {
        // Signal all non-conflicting queued thread-agnostic systems to start.
        for index in self.queued.difference(&self.thread_local) {
            if self.can_start_now(index) {
                let system = &self.parallel[index];
                system
                    .start_sender
                    .send(())
                    .await
                    .unwrap_or_else(|error| unreachable!(error));
                self.running.set(index, true);
                // Append this system's access information to the active access information.
                self.active_archetype_component_access
                    .extend(&system.archetype_component_access);
                self.active_resource_access.extend(&system.resource_access);
            }
        }
        // Remove running systems from queued systems.
        self.queued.difference_with(&self.running);
        // Avoid deadlocking if there's nothing to wait for.
        if 0 < self.running.count_ones(..) {
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
            self.update_counters_and_queue();
            // At least one system has finished, rebuild the active access information.
            self.active_archetype_component_access.clear();
            self.active_resource_access.clear();
            for index in self.running.ones() {
                self.active_archetype_component_access
                    .extend(&self.parallel[index].archetype_component_access);
                self.active_resource_access
                    .extend(&self.parallel[index].resource_access);
            }
        }
    }

    /// Drains `dependants_scratch`, decrementing dependency counters and queueing systems with
    /// satisfied dependencies if they should run this iteration.
    fn update_counters_and_queue(&mut self) {
        for index in self.dependants_scratch.drain(..) {
            if self.should_run[index] {
                let dependent = &mut self.parallel[index];
                dependent.dependencies_now -= 1;
                if dependent.dependencies_now == 0 {
                    self.queued.insert(index);
                }
            }
        }
    }
}

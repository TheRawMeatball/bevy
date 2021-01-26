mod executor;
mod executor_parallel;
mod stage;
mod state;
mod system_container;
mod system_descriptor;
mod system_set;

pub use executor::*;
pub use executor_parallel::*;
pub use stage::*;
pub use state::*;
pub use system_container::*;
pub use system_descriptor::*;
pub use system_set::*;

use crate::{
    ArchetypeComponent, BoxedSystem, IntoSystem, Resources, System, SystemId, TypeAccess, World,
};
use bevy_utils::{HashMap, HashSet};
use std::{any::TypeId, borrow::Cow, hash::Hash};

#[derive(Default)]
pub struct Schedule {
    stages: HashMap<String, Box<dyn Stage>>,
    stage_order: Vec<String>,
    run_criteria: RunCriteria,
}

impl Schedule {
    pub fn with_stage<S: Stage>(mut self, name: &str, stage: S) -> Self {
        self.add_stage(name, stage);
        self
    }

    pub fn with_stage_after<S: Stage>(mut self, target: &str, name: &str, stage: S) -> Self {
        self.add_stage_after(target, name, stage);
        self
    }

    pub fn with_stage_before<S: Stage>(mut self, target: &str, name: &str, stage: S) -> Self {
        self.add_stage_before(target, name, stage);
        self
    }

    pub fn with_run_criteria<S: System<In = (), Out = ShouldRun>>(mut self, system: S) -> Self {
        self.set_run_criteria(system);
        self
    }

    pub fn with_system_in_stage(
        mut self,
        stage_name: &'static str,
        system: impl Into<ParallelSystemDescriptor>,
    ) -> Self {
        self.add_system_to_stage(stage_name, system);
        self
    }

    pub fn with_exclusive_system_in_stage(
        mut self,
        stage_name: &'static str,
        system: impl Into<ExclusiveSystemDescriptor>,
    ) -> Self {
        self.add_exclusive_system_to_stage(stage_name, system);
        self
    }

    pub fn set_run_criteria<S: System<In = (), Out = ShouldRun>>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.run_criteria.set(Box::new(system.system()));
        self
    }

    pub fn add_stage<S: Stage>(&mut self, name: &str, stage: S) -> &mut Self {
        self.stage_order.push(name.to_string());
        self.stages.insert(name.to_string(), Box::new(stage));
        self
    }

    pub fn add_stage_after<S: Stage>(&mut self, target: &str, name: &str, stage: S) -> &mut Self {
        if self.stages.get(name).is_some() {
            panic!("Stage already exists: {}.", name);
        }

        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage_name)| *stage_name == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {}.", target));

        self.stages.insert(name.to_string(), Box::new(stage));
        self.stage_order.insert(target_index + 1, name.to_string());
        self
    }

    pub fn add_stage_before<S: Stage>(&mut self, target: &str, name: &str, stage: S) -> &mut Self {
        if self.stages.get(name).is_some() {
            panic!("Stage already exists: {}.", name);
        }

        let target_index = self
            .stage_order
            .iter()
            .enumerate()
            .find(|(_i, stage_name)| *stage_name == target)
            .map(|(i, _)| i)
            .unwrap_or_else(|| panic!("Target stage does not exist: {}.", target));

        self.stages.insert(name.to_string(), Box::new(stage));
        self.stage_order.insert(target_index, name.to_string());
        self
    }

    pub fn add_system_to_stage(
        &mut self,
        stage_name: &'static str,
        system: impl Into<ParallelSystemDescriptor>,
    ) -> &mut Self {
        let stage = self
            .get_stage_mut::<SystemStage>(stage_name)
            .unwrap_or_else(|| {
                panic!(
                    "Stage '{}' does not exist or is not a SystemStage",
                    stage_name
                )
            });
        stage.add_system(system);
        self
    }

    pub fn add_exclusive_system_to_stage(
        &mut self,
        stage_name: &'static str,
        system: impl Into<ExclusiveSystemDescriptor>,
    ) -> &mut Self {
        let stage = self
            .get_stage_mut::<SystemStage>(stage_name)
            .unwrap_or_else(|| {
                panic!(
                    "Stage '{}' does not exist or is not a SystemStage",
                    stage_name
                )
            });
        stage.add_exclusive_system(system);
        self
    }

    pub fn stage<T: Stage, F: FnOnce(&mut T) -> &mut T>(
        &mut self,
        name: &str,
        func: F,
    ) -> &mut Self {
        let stage = self
            .get_stage_mut::<T>(name)
            .unwrap_or_else(|| panic!("stage '{}' does not exist or is the wrong type", name));
        func(stage);
        self
    }

    pub fn get_stage<T: Stage>(&self, name: &str) -> Option<&T> {
        self.stages
            .get(name)
            .and_then(|stage| stage.downcast_ref::<T>())
    }

    pub fn get_stage_mut<T: Stage>(&mut self, name: &str) -> Option<&mut T> {
        self.stages
            .get_mut(name)
            .and_then(|stage| stage.downcast_mut::<T>())
    }

    pub fn run_once(&mut self, world: &mut World, resources: &mut Resources) {
        for name in self.stage_order.iter() {
            #[cfg(feature = "trace")]
            let stage_span = bevy_utils::tracing::info_span!("stage", name = name.as_str());
            #[cfg(feature = "trace")]
            let _stage_guard = stage_span.enter();
            let stage = self.stages.get_mut(name).unwrap();
            stage.run(world, resources);
        }
    }
}

impl Stage for Schedule {
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

pub fn clear_trackers_system(world: &mut World, resources: &mut Resources) {
    world.clear_trackers();
    resources.clear_trackers();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShouldRun {
    /// No, the system should not run
    No,
    /// Yes, the system should run
    Yes,
    /// Yes, the system should run and after running, the criteria should be checked again.
    YesAndLoop,
    /// No, the system should not run right now, but the criteria should be checked again later.
    NoAndLoop,
}

pub(crate) struct RunCriteria {
    criteria_system: Option<BoxedSystem<(), ShouldRun>>,
    initialized: bool,
}

impl Default for RunCriteria {
    fn default() -> Self {
        Self {
            criteria_system: None,
            initialized: false,
        }
    }
}

impl RunCriteria {
    pub fn set(&mut self, criteria_system: BoxedSystem<(), ShouldRun>) {
        self.criteria_system = Some(criteria_system);
        self.initialized = false;
    }

    pub fn should_run(&mut self, world: &mut World, resources: &mut Resources) -> ShouldRun {
        if let Some(ref mut run_criteria) = self.criteria_system {
            if !self.initialized {
                run_criteria.initialize(world, resources);
                self.initialized = true;
            }
            let should_run = run_criteria.run((), world, resources);
            run_criteria.apply_buffers(world, resources);
            // don't run when no result is returned or false is returned
            should_run.unwrap_or(ShouldRun::No)
        } else {
            ShouldRun::Yes
        }
    }
}

pub struct RunOnce {
    ran: bool,
    system_id: SystemId,
    archetype_component_access: TypeAccess<ArchetypeComponent>,
    component_access: TypeAccess<TypeId>,
    resource_access: TypeAccess<TypeId>,
}

impl Default for RunOnce {
    fn default() -> Self {
        Self {
            ran: false,
            system_id: SystemId::new(),
            archetype_component_access: Default::default(),
            component_access: Default::default(),
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

    fn component_access(&self) -> &TypeAccess<TypeId> {
        &self.component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn is_non_send(&self) -> bool {
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

    fn apply_buffers(&mut self, _world: &mut World, _resources: &mut Resources) {}

    fn initialize(&mut self, _world: &mut World, _resources: &mut Resources) {}
}

pub(crate) enum SortingResult<T> {
    Sorted(Vec<T>),
    FoundCycle(HashSet<T>),
}

pub(crate) fn topological_sorting<T>(graph: &HashMap<T, Vec<T>>) -> SortingResult<T>
where
    T: Hash + Eq + Clone,
{
    fn check_if_cycles_and_visit<N>(
        node: &N,
        graph: &HashMap<N, Vec<N>>,
        sorted: &mut Vec<N>,
        unvisited: &mut HashSet<N>,
        current: &mut HashSet<N>,
    ) -> bool
    where
        N: Hash + Eq + Clone,
    {
        if current.contains(node) {
            return true;
        } else if !unvisited.remove(node) {
            return false;
        }
        current.insert(node.clone());
        for node in graph.get(node).unwrap() {
            if check_if_cycles_and_visit(node, &graph, sorted, unvisited, current) {
                return true;
            }
        }
        sorted.push(node.clone());
        current.remove(node);
        false
    }
    let mut sorted = Vec::with_capacity(graph.len());
    let mut current = HashSet::with_capacity_and_hasher(graph.len(), Default::default());
    let mut unvisited = HashSet::with_capacity_and_hasher(graph.len(), Default::default());
    unvisited.extend(graph.keys().cloned());
    while let Some(node) = unvisited.iter().next().cloned() {
        if check_if_cycles_and_visit(&node, graph, &mut sorted, &mut unvisited, &mut current) {
            return SortingResult::FoundCycle(current);
        }
    }
    SortingResult::Sorted(sorted)
}

// TODO more relevant tests
/*#[test]
fn schedule() {
    let mut world = World::new();
    let mut resources = Resources::default();
    resources.insert(ComputeTaskPool(TaskPool::default()));
    resources.insert(CompletedSystems::default());
    resources.insert(1.0f64);
    resources.insert(2isize);

    world.spawn((1.0f32,));
    world.spawn((1u32, 1u64));
    world.spawn((2u32,));

    let mut stage_a = SystemStage::parallel(); // component queries
    let mut stage_b = SystemStage::parallel(); // thread local
    let mut stage_c = SystemStage::parallel(); // resources

    // A system names
    const READ_U32_SYSTEM_NAME: &str = "read_u32";
    const WRITE_FLOAT_SYSTEM_NAME: &str = "write_float";
    const READ_U32_WRITE_U64_SYSTEM_NAME: &str = "read_u32_write_u64";
    const READ_U64_SYSTEM_NAME: &str = "read_u64";

    // B system names
    const WRITE_U64_SYSTEM_NAME: &str = "write_u64";
    const THREAD_LOCAL_SYSTEM_SYSTEM_NAME: &str = "thread_local_system";
    const WRITE_F32_SYSTEM_NAME: &str = "write_f32";

    // C system names
    const READ_F64_RES_SYSTEM_NAME: &str = "read_f64_res";
    const READ_ISIZE_RES_SYSTEM_NAME: &str = "read_isize_res";
    const READ_ISIZE_WRITE_F64_RES_SYSTEM_NAME: &str = "read_isize_write_f64_res";
    const WRITE_F64_RES_SYSTEM_NAME: &str = "write_f64_res";

    // A systems

    fn read_u32(completed_systems: Res<CompletedSystems>, _query: Query<&u32>) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        completed_systems.insert(READ_U32_SYSTEM_NAME);
    }

    fn write_float(completed_systems: Res<CompletedSystems>, _query: Query<&f32>) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        completed_systems.insert(WRITE_FLOAT_SYSTEM_NAME);
    }

    fn read_u32_write_u64(
        completed_systems: Res<CompletedSystems>,
        _query: Query<(&u32, &mut u64)>,
    ) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        assert!(!completed_systems.contains(READ_U64_SYSTEM_NAME));
        completed_systems.insert(READ_U32_WRITE_U64_SYSTEM_NAME);
    }

    fn read_u64(completed_systems: Res<CompletedSystems>, _query: Query<&u64>) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        assert!(completed_systems.contains(READ_U32_WRITE_U64_SYSTEM_NAME));
        assert!(!completed_systems.contains(WRITE_U64_SYSTEM_NAME));
        completed_systems.insert(READ_U64_SYSTEM_NAME);
    }

    stage_a.add_system(read_u32.system());
    stage_a.add_system(write_float.system());
    stage_a.add_system(read_u32_write_u64.system());
    stage_a.add_system(read_u64.system());

    // B systems

    fn write_u64(completed_systems: Res<CompletedSystems>, _query: Query<&mut u64>) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        assert!(completed_systems.contains(READ_U64_SYSTEM_NAME));
        assert!(!completed_systems.contains(THREAD_LOCAL_SYSTEM_SYSTEM_NAME));
        assert!(!completed_systems.contains(WRITE_F32_SYSTEM_NAME));
        completed_systems.insert(WRITE_U64_SYSTEM_NAME);
    }

    fn thread_local_system(_world: &mut World, resources: &mut Resources) {
        let completed_systems = resources.get::<CompletedSystems>().unwrap();
        let mut completed_systems = completed_systems.completed_systems.lock();
        assert!(completed_systems.contains(WRITE_U64_SYSTEM_NAME));
        assert!(!completed_systems.contains(WRITE_F32_SYSTEM_NAME));
        completed_systems.insert(THREAD_LOCAL_SYSTEM_SYSTEM_NAME);
    }

    fn write_f32(completed_systems: Res<CompletedSystems>, _query: Query<&mut f32>) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        assert!(completed_systems.contains(WRITE_U64_SYSTEM_NAME));
        assert!(completed_systems.contains(THREAD_LOCAL_SYSTEM_SYSTEM_NAME));
        assert!(!completed_systems.contains(READ_F64_RES_SYSTEM_NAME));
        completed_systems.insert(WRITE_F32_SYSTEM_NAME);
    }

    stage_b.add_system(write_u64.system());
    stage_b.add_system(thread_local_system.system());
    stage_b.add_system(write_f32.system());

    // C systems

    fn read_f64_res(completed_systems: Res<CompletedSystems>, _f64_res: Res<f64>) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        assert!(completed_systems.contains(WRITE_F32_SYSTEM_NAME));
        assert!(!completed_systems.contains(READ_ISIZE_WRITE_F64_RES_SYSTEM_NAME));
        assert!(!completed_systems.contains(WRITE_F64_RES_SYSTEM_NAME));
        completed_systems.insert(READ_F64_RES_SYSTEM_NAME);
    }

    fn read_isize_res(completed_systems: Res<CompletedSystems>, _isize_res: Res<isize>) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        completed_systems.insert(READ_ISIZE_RES_SYSTEM_NAME);
    }

    fn read_isize_write_f64_res(
        completed_systems: Res<CompletedSystems>,
        _isize_res: Res<isize>,
        _f64_res: ResMut<f64>,
    ) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        assert!(completed_systems.contains(READ_F64_RES_SYSTEM_NAME));
        assert!(!completed_systems.contains(WRITE_F64_RES_SYSTEM_NAME));
        completed_systems.insert(READ_ISIZE_WRITE_F64_RES_SYSTEM_NAME);
    }

    fn write_f64_res(completed_systems: Res<CompletedSystems>, _f64_res: ResMut<f64>) {
        let mut completed_systems = completed_systems.completed_systems.lock();
        assert!(completed_systems.contains(READ_F64_RES_SYSTEM_NAME));
        assert!(completed_systems.contains(READ_ISIZE_WRITE_F64_RES_SYSTEM_NAME));
        completed_systems.insert(WRITE_F64_RES_SYSTEM_NAME);
    }

    stage_c.add_system(read_f64_res.system());
    stage_c.add_system(read_isize_res.system());
    stage_c.add_system(read_isize_write_f64_res.system());
    stage_c.add_system(write_f64_res.system());

    fn run_and_validate(schedule: &mut Schedule, world: &mut World, resources: &mut Resources) {
        schedule.initialize_and_run(world, resources);

        let stage_a = schedule.get_stage::<SystemStage>("a").unwrap();
        let stage_b = schedule.get_stage::<SystemStage>("b").unwrap();
        let stage_c = schedule.get_stage::<SystemStage>("c").unwrap();

        let a_executor = stage_a
            .get_executor::<ParallelSystemStageExecutor>()
            .unwrap();
        let b_executor = stage_b
            .get_executor::<ParallelSystemStageExecutor>()
            .unwrap();
        let c_executor = stage_c
            .get_executor::<ParallelSystemStageExecutor>()
            .unwrap();

        assert_eq!(
            a_executor.system_dependents(),
            vec![vec![], vec![], vec![3], vec![]]
        );
        assert_eq!(
            b_executor.system_dependents(),
            vec![vec![1], vec![2], vec![]]
        );
        assert_eq!(
            c_executor.system_dependents(),
            vec![vec![2, 3], vec![], vec![3], vec![]]
        );

        let stage_a_len = a_executor.system_dependencies().len();
        let mut read_u64_deps = FixedBitSet::with_capacity(stage_a_len);
        read_u64_deps.insert(2);

        assert_eq!(
            a_executor.system_dependencies(),
            vec![
                FixedBitSet::with_capacity(stage_a_len),
                FixedBitSet::with_capacity(stage_a_len),
                FixedBitSet::with_capacity(stage_a_len),
                read_u64_deps,
            ]
        );

        let stage_b_len = b_executor.system_dependencies().len();
        let mut thread_local_deps = FixedBitSet::with_capacity(stage_b_len);
        thread_local_deps.insert(0);
        let mut write_f64_deps = FixedBitSet::with_capacity(stage_b_len);
        write_f64_deps.insert(1);
        assert_eq!(
            b_executor.system_dependencies(),
            vec![
                FixedBitSet::with_capacity(stage_b_len),
                thread_local_deps,
                write_f64_deps
            ]
        );

        let stage_c_len = c_executor.system_dependencies().len();
        let mut read_isize_write_f64_res_deps = FixedBitSet::with_capacity(stage_c_len);
        read_isize_write_f64_res_deps.insert(0);
        let mut write_f64_res_deps = FixedBitSet::with_capacity(stage_c_len);
        write_f64_res_deps.insert(0);
        write_f64_res_deps.insert(2);
        assert_eq!(
            c_executor.system_dependencies(),
            vec![
                FixedBitSet::with_capacity(stage_c_len),
                FixedBitSet::with_capacity(stage_c_len),
                read_isize_write_f64_res_deps,
                write_f64_res_deps
            ]
        );

        let completed_systems = resources.get::<CompletedSystems>().unwrap();
        assert_eq!(
            completed_systems.completed_systems.lock().len(),
            11,
            "completed_systems should have been incremented once for each system"
        );
    }

    let mut schedule = Schedule::default();
    schedule.add_stage("a", stage_a);
    schedule.add_stage("b", stage_b);
    schedule.add_stage("c", stage_c);

    // Test the "clean start" case
    run_and_validate(&mut schedule, &mut world, &mut resources);

    // Stress test the "continue running" case
    for _ in 0..1000 {
        // run again (with completed_systems reset) to ensure executor works correctly across runs
        resources
            .get::<CompletedSystems>()
            .unwrap()
            .completed_systems
            .lock()
            .clear();
        run_and_validate(&mut schedule, &mut world, &mut resources);
    }
}*/

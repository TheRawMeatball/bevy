use crate::{ParallelSystemDescriptor, Resource, Resources, Stage, SystemStage, World};
use bevy_utils::HashMap;
use std::{mem::Discriminant, ops::Deref};
use thiserror::Error;

pub(crate) struct StateStages {
    update: Box<dyn Stage>,
    enter: Box<dyn Stage>,
    exit: Box<dyn Stage>,
}

impl Default for StateStages {
    fn default() -> Self {
        Self {
            enter: Box::new(SystemStage::parallel()),
            update: Box::new(SystemStage::parallel()),
            exit: Box::new(SystemStage::parallel()),
        }
    }
}

pub struct StateStage<T> {
    stages: HashMap<Discriminant<T>, StateStages>,
}

impl<T> Default for StateStage<T> {
    fn default() -> Self {
        Self {
            stages: Default::default(),
        }
    }
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T> StateStage<T> {
    pub fn with_enter_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_enter_stage(state, stage);
        self
    }

    pub fn with_exit_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_exit_stage(state, stage);
        self
    }

    pub fn with_update_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_update_stage(state, stage);
        self
    }

    pub fn set_enter_stage<S: Stage>(&mut self, state: T, stage: S) -> &mut Self {
        let stages = self.state_stages(state);
        stages.enter = Box::new(stage);
        self
    }

    pub fn set_exit_stage<S: Stage>(&mut self, state: T, stage: S) -> &mut Self {
        let stages = self.state_stages(state);
        stages.exit = Box::new(stage);
        self
    }

    pub fn set_update_stage<S: Stage>(&mut self, state: T, stage: S) -> &mut Self {
        let stages = self.state_stages(state);
        stages.update = Box::new(stage);
        self
    }

    pub fn on_state_enter(
        &mut self,
        state: T,
        system: impl Into<ParallelSystemDescriptor>,
    ) -> &mut Self {
        self.enter_stage(state, |system_stage: &mut SystemStage| {
            system_stage.add_system(system)
        })
    }

    pub fn on_state_exit(
        &mut self,
        state: T,
        system: impl Into<ParallelSystemDescriptor>,
    ) -> &mut Self {
        self.exit_stage(state, |system_stage: &mut SystemStage| {
            system_stage.add_system(system)
        })
    }

    pub fn on_state_update(
        &mut self,
        state: T,
        system: impl Into<ParallelSystemDescriptor>,
    ) -> &mut Self {
        self.update_stage(state, |system_stage: &mut SystemStage| {
            system_stage.add_system(system)
        })
    }

    pub fn enter_stage<S: Stage, F: FnOnce(&mut S) -> &mut S>(
        &mut self,
        state: T,
        func: F,
    ) -> &mut Self {
        let stages = self.state_stages(state);
        func(
            stages
                .enter
                .downcast_mut()
                .expect("'Enter' stage does not match the given type"),
        );
        self
    }

    pub fn exit_stage<S: Stage, F: FnOnce(&mut S) -> &mut S>(
        &mut self,
        state: T,
        func: F,
    ) -> &mut Self {
        let stages = self.state_stages(state);
        func(
            stages
                .exit
                .downcast_mut()
                .expect("'Exit' stage does not match the given type"),
        );
        self
    }

    pub fn update_stage<S: Stage, F: FnOnce(&mut S) -> &mut S>(
        &mut self,
        state: T,
        func: F,
    ) -> &mut Self {
        let stages = self.state_stages(state);
        func(
            stages
                .update
                .downcast_mut()
                .expect("'Update' stage does not match the given type"),
        );
        self
    }

    fn state_stages(&mut self, state: T) -> &mut StateStages {
        self.stages
            .entry(std::mem::discriminant(&state))
            .or_default()
    }
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T: Resource + Clone> Stage for StateStage<T> {
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        let current_stage = loop {
            let (next_stage, current_stage) = {
                let mut state = resources
                    .get_mut::<State<T>>()
                    .expect("Missing state resource");
                let result = (
                    state.next.as_ref().map(|next| std::mem::discriminant(next)),
                    std::mem::discriminant(&state.current),
                );

                state.apply_next();

                result
            };

            // if next_stage is Some, we just applied a new state
            if let Some(next_stage) = next_stage {
                if next_stage != current_stage {
                    if let Some(current_state_stages) = self.stages.get_mut(&current_stage) {
                        current_state_stages.exit.run(world, resources);
                    }
                }

                if let Some(next_state_stages) = self.stages.get_mut(&next_stage) {
                    next_state_stages.enter.run(world, resources);
                }
            } else {
                break current_stage;
            }
        };

        if let Some(current_state_stages) = self.stages.get_mut(&current_stage) {
            current_state_stages.update.run(world, resources);
        }
    }
}
#[derive(Debug, Error)]
pub enum StateError {
    #[error("Attempted to change the state to the current state.")]
    AlreadyInState,
    #[error("Attempted to queue a state change, but there was already a state queued.")]
    StateAlreadyQueued,
}

#[derive(Debug)]
pub struct State<T: Clone> {
    previous: Option<T>,
    current: T,
    next: Option<T>,
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T: Clone> State<T> {
    pub fn new(state: T) -> Self {
        Self {
            current: state.clone(),
            previous: None,
            // add value to queue so that we "enter" the state
            next: Some(state),
        }
    }

    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn previous(&self) -> Option<&T> {
        self.previous.as_ref()
    }

    pub fn next(&self) -> Option<&T> {
        self.next.as_ref()
    }

    /// Queue a state change. This will fail if there is already a state in the queue, or if the given `state` matches the current state
    pub fn set_next(&mut self, state: T) -> Result<(), StateError> {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        if self.next.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        self.next = Some(state);
        Ok(())
    }

    /// Same as [Self::queue], but if there is already a next state, it will be overwritten instead of failing
    pub fn overwrite_next(&mut self, state: T) -> Result<(), StateError> {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        self.next = Some(state);
        Ok(())
    }

    fn apply_next(&mut self) {
        if let Some(next) = self.next.take() {
            let previous = std::mem::replace(&mut self.current, next);
            if std::mem::discriminant(&previous) != std::mem::discriminant(&self.current) {
                self.previous = Some(previous)
            }
        }
    }
}

#[allow(clippy::mem_discriminant_non_enum)]
mod alternate {
    use std::{
        any::TypeId,
        marker::PhantomData,
        mem::{discriminant, Discriminant},
    };

    use crate::{
        ArchetypeComponent, ResMut, Resource, ShouldRun, State, System, SystemId, TypeAccess,
    };

    impl<T: Clone + Resource> State<T> {
        pub fn on_update(val: T) -> impl System<In = (), Out = ShouldRun> {
            Wrapper::<T, OnUpdate>::new(discriminant(&val))
        }

        pub fn on_entry(val: T) -> impl System<In = (), Out = ShouldRun> {
            Wrapper::<T, OnEntry>::new(discriminant(&val))
        }

        pub fn on_exit(val: T) -> impl System<In = (), Out = ShouldRun> {
            Wrapper::<T, OnExit>::new(discriminant(&val))
        }

        // TODO: Add a metod to AppBuilder that adds this system and the necessary resource
        pub fn update(mut state: ResMut<State<T>>) {
            state.previous.take();
            if let Some(next) = state.next.take() {
                state.previous = Some(std::mem::replace(&mut state.current, next));
            }
        }
    }

    trait Comparer<T: Clone> {
        fn compare(d: Discriminant<T>, s: &State<T>) -> bool;
    }

    struct OnUpdate;
    impl<T: Clone> Comparer<T> for OnUpdate {
        fn compare(d: Discriminant<T>, s: &State<T>) -> bool {
            discriminant(&s.current) == d
        }
    }
    struct OnEntry;
    impl<T: Clone> Comparer<T> for OnEntry {
        fn compare(d: Discriminant<T>, s: &State<T>) -> bool {
            s.next().map_or(false, |n| discriminant(n) == d)
        }
    }
    struct OnExit;
    impl<T: Clone> Comparer<T> for OnExit {
        fn compare(d: Discriminant<T>, s: &State<T>) -> bool {
            s.next().is_some() && discriminant(&s.current) == d
        }
    }

    impl<T: Clone + Resource, C: Comparer<T>> Wrapper<T, C> {
        fn new(discriminant: Discriminant<T>) -> Self {
            let mut resource_access = TypeAccess::default();
            resource_access.add_read(std::any::TypeId::of::<State<T>>());
            Self {
                discriminant,
                resource_access,
                id: SystemId::new(),
                archetype_access: Default::default(),
                marker: Default::default(),
            }
        }
    }

    struct Wrapper<T: Clone + Resource, C: Comparer<T>> {
        discriminant: Discriminant<T>,
        resource_access: TypeAccess<TypeId>,
        id: SystemId,
        archetype_access: TypeAccess<ArchetypeComponent>,
        marker: PhantomData<C>,
    }

    impl<T: Clone + Resource, C: Comparer<T> + Resource> System for Wrapper<T, C> {
        type In = ();
        type Out = ShouldRun;

        fn name(&self) -> std::borrow::Cow<'static, str> {
            std::borrow::Cow::Owned(format!(
                "State checker for state {}",
                std::any::type_name::<T>()
            ))
        }

        fn id(&self) -> crate::SystemId {
            self.id
        }

        fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
            &self.archetype_access
        }

        fn resource_access(&self) -> &TypeAccess<std::any::TypeId> {
            &self.resource_access
        }

        fn is_thread_local(&self) -> bool {
            false
        }

        unsafe fn run_unsafe(
            &mut self,
            _input: Self::In,
            _world: &crate::World,
            resources: &crate::Resources,
        ) -> Option<Self::Out> {
            Some(
                if C::compare(self.discriminant, &*resources.get::<State<T>>().unwrap()) {
                    ShouldRun::Yes
                } else {
                    ShouldRun::No
                },
            )
        }

        fn update_access(&mut self, _world: &crate::World) {}

        fn apply_buffers(&mut self, _world: &mut crate::World, _resources: &mut crate::Resources) {}

        fn initialize(&mut self, _world: &mut crate::World, _resources: &mut crate::Resources) {}
    }
}

impl<T: Clone> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

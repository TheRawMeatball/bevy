use async_channel::{Receiver, Sender};
use parking_lot::Mutex;
use std::{any::TypeId, borrow::Cow, marker::PhantomData, ops::Deref, sync::Arc};

use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_utils::BoxedFuture;

use crate::{
    ArchetypeComponent, BoxedSystem, FetchSystemParam, Resources, System, SystemId, SystemParam,
    SystemState, TypeAccess, World,
};

// this is stable on nightly, and will land on 2021-03-25 :)
pub trait AsyncSystem<Trigger, Params, Future, Marker, const ACCESSOR_COUNT: usize>: Sized {
    type TaskPool: Deref<Target = TaskPool>;
    type Params;

    fn systems(self) -> ([BoxedSystem; ACCESSOR_COUNT], Sender<Trigger>);
}
pub struct Accessor<P: SystemParam> {
    channel: Sender<Box<dyn FnOnce(&SystemState, &World, &Resources) + Send + Sync>>,
    _marker: OpaquePhantomData<P>,
}

impl<P: SystemParam> Accessor<P> {
    pub async fn access<F, R: Send + 'static>(&mut self, sync: F) -> R
    where
        // Removing the 'static here would allow removing the
        // transmutes, but its currently not possible due to an ICE.
        F: FnOnce(<P::Fetch as FetchSystemParam<'static>>::Item) -> R + Send + Sync + 'static,
    {
        let (tx, rx) = async_channel::bounded(1);
        self.channel
            .send(Box::new(move |state, world, resources| {
                // Safe: the sent closure is executed inside run_unsafe, which provides the correct guarantees.
                match unsafe {
                    P::Fetch::get_param(
                        std::mem::transmute::<_, &'static _>(state),
                        std::mem::transmute::<_, &'static _>(world),
                        std::mem::transmute::<_, &'static _>(resources),
                    )
                } {
                    Some(params) => tx.try_send(sync(params)).unwrap(),
                    None => (),
                }
            }))
            .await
            .unwrap();
        rx.recv().await.unwrap()
    }
}

pub struct AccessorRunnerSystem<P: SystemParam> {
    state: SystemState,
    channel: Receiver<Box<dyn FnOnce(&SystemState, &World, &Resources) + Send + Sync>>,
    core: Arc<Mutex<Option<BoxedFuture<'static, ()>>>>,
    _marker: OpaquePhantomData<P>,
}

struct OpaquePhantomData<T> {
    _phantom: PhantomData<T>,
}

unsafe impl<T> Send for OpaquePhantomData<T> {}
unsafe impl<T> Sync for OpaquePhantomData<T> {}

impl<T> Default for OpaquePhantomData<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<P: SystemParam + 'static> System for AccessorRunnerSystem<P> {
    type In = ();
    type Out = ();

    fn name(&self) -> Cow<'static, str> {
        self.state.name.clone()
    }

    fn id(&self) -> SystemId {
        self.state.id
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.state.archetype_component_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.state.resource_access
    }

    unsafe fn run_unsafe(
        &mut self,
        _: Self::In,
        world: &World,
        resources: &Resources,
    ) -> Option<Self::Out> {
        match self.channel.try_recv() {
            Ok(sync) => (sync)(&self.state, world, resources),
            Err(async_channel::TryRecvError::Closed) => panic!(),
            _ => (),
        }

        Some(())
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        <P::Fetch as FetchSystemParam>::init(&mut self.state, world, resources);
        if let Some(f) = self.core.lock().take() {
            let executor = resources.get_mut::<AsyncComputeTaskPool>().unwrap();
            executor.spawn(f).detach();
        }
    }

    fn apply_buffers(&mut self, world: &mut World, resources: &mut Resources) {
        self.state.commands.get_mut().apply(world, resources);
        if let Some(ref commands) = self.state.arc_commands {
            let mut commands = commands.lock();
            commands.apply(world, resources);
        }
    }

    fn update_access(&mut self, world: &World) {
        self.state.update(world);
    }

    fn component_access(&self) -> &TypeAccess<TypeId> {
        &self.state.component_access
    }

    fn is_non_send(&self) -> bool {
        self.state.is_non_send
    }
}

// Implements AsyncSystem for async functions with up to 6 different accessors
#[doc(hidden)]
pub mod impls {
    use crate::In;

    use super::*;
    use std::{future::Future, pin::Pin};

    pub struct SimpleAsyncMarker;
    pub struct InAsyncMarker;

    macro_rules! impl_async_system {
        ($param_count: literal, $([$i: ident, $tx: ident, $rx: ident]),*) => {
            impl<Func, $($i,)* Fut> AsyncSystem<(), ($($i,)*), Fut, SimpleAsyncMarker, $param_count> for Func
            where
                Func: FnMut($(Accessor<$i>,)*) -> Fut + Send + 'static,
                Fut: Future<Output = ()> + Send + 'static,
                $($i: SystemParam + 'static,)*
            {
                type TaskPool = AsyncComputeTaskPool;
                type Params = ($($i,)*);
                fn systems(mut self) -> ([BoxedSystem; $param_count], Sender<()>) {
                    $(let ($tx, $rx) = async_channel::unbounded();)*
                    let (tx, rx) = async_channel::unbounded();
                    let f = async move {
                        loop {
                            rx.recv().await.unwrap();
                            (self)(
                                $(
                                    Accessor {
                                        channel: $tx.clone(),
                                        _marker: Default::default(),
                                    },
                                )*
                            )
                            .await;
                        }
                    };
                    let arc = Arc::new(Mutex::new(Some(
                        Box::pin(f) as Pin<Box<dyn Future<Output = ()> + Send>>
                    )));
                    ([$(
                        Box::new(AccessorRunnerSystem::<$i> {
                            state: {
                                let mut resource_access = TypeAccess::default();
                                resource_access.add_write(TypeId::of::<Self::TaskPool>());
                                SystemState {
                                    name: std::any::type_name::<Self>().into(),
                                    archetype_component_access: TypeAccess::default(),
                                    component_access: TypeAccess::default(),
                                    resource_access,
                                    is_non_send: false,
                                    local_resource_access: TypeAccess::default(),
                                    id: SystemId::new(),
                                    commands: Default::default(),
                                    arc_commands: Default::default(),
                                    current_query_index: Default::default(),
                                    query_archetype_component_accesses: Vec::new(),
                                    query_accesses: Vec::new(),
                                    query_type_names: Vec::new(),
                                }
                            },
                            channel: $rx,
                            core: arc.clone(),
                            _marker: Default::default(),
                        }),
                    )*], tx)
                }
            }

            impl<Trigger, Func, $($i,)* Fut> AsyncSystem<Trigger, ($($i,)*), Fut, InAsyncMarker, $param_count> for Func
            where
                Trigger: Send + Sync + 'static,
                Func: FnMut(In<Trigger>, $(Accessor<$i>,)*) -> Fut + Send + 'static,
                Fut: Future<Output = ()> + Send + 'static,
                $($i: SystemParam + 'static,)*
            {
                type TaskPool = AsyncComputeTaskPool;
                type Params = ($($i,)*);
                fn systems(mut self) -> ([BoxedSystem; $param_count], Sender<Trigger>) {
                    $(let ($tx, $rx) = async_channel::unbounded();)*
                    let (tx, rx) = async_channel::unbounded();
                    let f = async move {
                        loop {
                            (self)(
                                In(rx.recv().await.unwrap()),
                                $(
                                    Accessor {
                                        channel: $tx.clone(),
                                        _marker: Default::default(),
                                    },
                                )*
                            )
                            .await;
                        }
                    };
                    let arc = Arc::new(Mutex::new(Some(
                        Box::pin(f) as Pin<Box<dyn Future<Output = ()> + Send>>
                    )));
                    ([$(
                        Box::new(AccessorRunnerSystem::<$i> {
                            state: {
                                let mut resource_access = TypeAccess::default();
                                resource_access.add_write(TypeId::of::<Self::TaskPool>());
                                SystemState {
                                    name: std::any::type_name::<Self>().into(),
                                    archetype_component_access: TypeAccess::default(),
                                    component_access: TypeAccess::default(),
                                    resource_access,
                                    is_non_send: false,
                                    local_resource_access: TypeAccess::default(),
                                    id: SystemId::new(),
                                    commands: Default::default(),
                                    arc_commands: Default::default(),
                                    current_query_index: Default::default(),
                                    query_archetype_component_accesses: Vec::new(),
                                    query_accesses: Vec::new(),
                                    query_type_names: Vec::new(),
                                }
                            },
                            channel: $rx,
                            core: arc.clone(),
                            _marker: Default::default(),
                        }),
                    )*], tx)
                }
            }
        };
    }

    impl_async_system!(1, [A, txa, rxa]);
    impl_async_system!(2, [A, txa, rxa], [B, txb, rxb]);
    impl_async_system!(3, [A, txa, rxa], [B, txb, rxb], [C, txc, rxc]);
    impl_async_system!(
        4,
        [A, txa, rxa],
        [B, txb, rxb],
        [C, txc, rxc],
        [D, txd, rxd]
    );
    impl_async_system!(
        5,
        [A, txa, rxa],
        [B, txb, rxb],
        [C, txc, rxc],
        [D, txd, rxd],
        [E, txe, rxe]
    );
    impl_async_system!(
        6,
        [A, txa, rxa],
        [B, txb, rxb],
        [C, txc, rxc],
        [D, txd, rxd],
        [E, txe, rxe],
        [F, txf, rxf]
    );

    pub trait SimpleAsyncSystem<P, F>
    where
        P: SystemParam,
    {
        fn system(self) -> AccessorRunnerSystem<P>;
    }

    impl<Func, P, Fut> SimpleAsyncSystem<P, Fut> for Func
    where
        Func: FnMut(Accessor<P>) -> Fut + Send + 'static,
        P: SystemParam + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        fn system(mut self) -> AccessorRunnerSystem<P> {
            let (txf, rxf) = async_channel::unbounded();
            let f = async move {
                loop {
                    (self)(Accessor {
                        channel: txf.clone(),
                        _marker: Default::default(),
                    })
                    .await;
                }
            };
            let arc = Arc::new(Mutex::new(Some(
                Box::pin(f) as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            )));
            AccessorRunnerSystem {
                state: {
                    let mut resource_access = TypeAccess::default();
                    resource_access.add_write(TypeId::of::<AsyncComputeTaskPool>());
                    SystemState {
                        name: std::any::type_name::<Self>().into(),
                        archetype_component_access: TypeAccess::default(),
                        component_access: TypeAccess::default(),
                        resource_access,
                        is_non_send: false,
                        local_resource_access: TypeAccess::default(),
                        id: SystemId::new(),
                        commands: Default::default(),
                        arc_commands: Default::default(),
                        current_query_index: Default::default(),
                        query_archetype_component_accesses: Vec::new(),
                        query_accesses: Vec::new(),
                        query_type_names: Vec::new(),
                    }
                },
                channel: rxf,
                core: arc,
                _marker: Default::default(),
            }
        }
    }
}
pub use impls::SimpleAsyncSystem;

#[cfg(test)]
mod test {
    use bevy_tasks::{AsyncComputeTaskPool, TaskPoolBuilder};

    use super::{Accessor, AsyncSystem, SimpleAsyncSystem};

    use crate::{
        Commands, IntoSystem, ParallelSystemDescriptorCoercion, Query, Res, ResMut, Resources,
        Stage, SystemStage, World,
    };

    async fn complex_async_system(
        mut access_1: Accessor<(Res<'_, u32>, ResMut<'_, String>)>,
        mut access_2: Accessor<Res<'_, String>>,
    ) {
        loop {
            let mut x = None;
            let returns = access_1
                .access(move |(number, mut res)| {
                    //
                    *res = "Hi!".to_owned();
                    assert_eq!(x, None);
                    x = Some(*number);
                    x
                })
                .await;
            assert_eq!(returns, Some(3));

            access_2
                .access(|res| {
                    assert_eq!("Hi!", &*res);
                })
                .await;
        }
    }

    async fn simple_async_system(mut accessor: Accessor<Query<'_, (&u32, &i64)>>) {
        accessor
            .access(|query| {
                for res in query.iter() {
                    match res {
                        (3, 5) | (7, -8) => (),
                        _ => unreachable!(),
                    }
                }
            })
            .await;
    }

    #[test]
    fn run_async_system() {
        let mut world = World::new();
        let mut resources = Resources::default();

        let mut commands = Commands::default();
        commands.set_entity_reserver(world.get_entity_reserver());

        commands
            .spawn((3u32, 5i64))
            .spawn((7u32, -8i64))
            .insert_resource("Hello".to_owned())
            .insert_resource(3u32)
            .insert_resource(AsyncComputeTaskPool(
                TaskPoolBuilder::default()
                    .thread_name("Async Compute Task Pool".to_string())
                    .build(),
            ));

        commands.apply(&mut world, &mut resources);

        let ([sync_1, sync_2], fire_sender) = complex_async_system.systems();
        fire_sender.try_send(()).unwrap();

        let mut stage = SystemStage::parallel();
        stage
            .add_system(
                (|string: Res<String>| {
                    assert_eq!("Hello", &*string);
                })
                .system()
                .label("1"),
            )
            .add_system(sync_1.label("2").after("1"))
            .add_system(
                (|string: Res<String>| {
                    assert_eq!("Hi!", &*string);
                })
                .system()
                .label("3")
                .after("2"),
            )
            .add_system(sync_2.label("4").after("3"))
            .add_system(simple_async_system.system().after("4"));

        stage.run(&mut world, &mut resources);
    }
}

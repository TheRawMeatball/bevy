use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::DerefMut;

use super::{IntoRunCriteria, RunCriteriaDescriptor, RunCriteriaDescriptorCoercion, ShouldRun};
use crate::schedule::label::RunCriteriaLabel;
use crate::{
    component::Component,
    prelude::{EventReader, In, IntoChainSystem, IntoSystem, Local, Res, ResMut, System},
    system::Required,
};

#[derive(Clone, Copy)]
struct PatternLiteral<T>(fn(&T) -> bool, &'static str);
impl<T> PartialEq for PatternLiteral<T> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl<T> PatternLiteral<T> {
    fn matches(&self, t: &T) -> bool {
        self.0(t)
    }
}

macro_rules! pl {
    ($pat:pat) => {
        PatternLiteral(|val| matches!(val, $pat), stringify!($pat))
    };
}

struct State<T: Component + Clone> {
    current: T,
}

struct StateChange<T: Component + Clone> {
    f: fn(T) -> T,
    silent: bool,
}

struct StateScratchSpace<T: Component + Clone> {
    _marker: PhantomData<T>,
    prepare_for_exit: bool,
    done: bool,
    transition: Transition<T>,
}

enum Transition<T> {
    None,
    Enter { exiting: T, silent: bool },
    Exit { entering: T, silent: bool },
}

impl<T> Transition<T> {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Transition::None)
    }
}

struct DriverLabel<T: Component>(PhantomData<T>);
impl<T: Component> Hash for DriverLabel<T> {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl<T: Component> PartialEq for DriverLabel<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl<T: Component> Eq for DriverLabel<T> {}
impl<T: Component> Debug for DriverLabel<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
impl<T: Component> Clone for DriverLabel<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Component> Copy for DriverLabel<T> {}

impl<T: Component> RunCriteriaLabel for DriverLabel<T> {
    fn dyn_clone(&self) -> Box<dyn RunCriteriaLabel> {
        Box::new(self.clone())
    }
}
fn state_driver<T: Component + Clone>(
    mut state: ResMut<State<T>>,
    mut scratch: ResMut<StateScratchSpace<T>>,
    mut er: EventReader<StateChange<T>>,
) -> ShouldRun {
    match scratch.transition.take() {
        Transition::None => {
            if let Some(next) = er.iter().next() {
                scratch.transition = Transition::Exit {
                    entering: (next.f)(state.current.clone()),
                    silent: next.silent,
                }
            } else if scratch.done {
                scratch.done = false;
                return ShouldRun::No;
            } else {
                if scratch.prepare_for_exit {
                    scratch.prepare_for_exit = false;
                    scratch.done = true;
                } else {
                    scratch.prepare_for_exit = true;
                }
            }
        }
        Transition::Enter { silent, .. } => {
            scratch.prepare_for_exit = true;
            if silent {
                return state_driver(state, scratch, er);
            }
            scratch.transition = Transition::None;
        }
        Transition::Exit { entering, silent } => {
            scratch.prepare_for_exit = false;
            scratch.transition = Transition::Enter {
                exiting: std::mem::replace(&mut state.current, entering),
                silent,
            };
        }
    }

    ShouldRun::NoAndCheckAgain
}

fn make_state_driver<T: Component + Clone>() -> RunCriteriaDescriptor {
    state_driver::<T>
        .system()
        .label(DriverLabel::<T>(PhantomData))
}

fn on_update<T: Component + Clone>(state: PatternLiteral<T>) -> RunCriteriaDescriptor {
    (|current: Res<State<T>>,
      scratch: Res<StateScratchSpace<T>>,
      state: Required<PatternLiteral<T>>| {
        state.matches(&current.current) && matches!(&scratch.transition, Transition::None)
    })
    .system()
    .config(|(_, _, s)| *s = Some(state))
    .chain(should_run_adapter::<T>.system())
    .after(DriverLabel::<T>(PhantomData))
}

fn on_enter<T: Component + Clone>(state: PatternLiteral<T>) -> RunCriteriaDescriptor {
    (|current: Res<State<T>>,
      scratch: Res<StateScratchSpace<T>>,
      state: Required<PatternLiteral<T>>| {
        state.matches(&current.current)
            && matches!(&scratch.transition, Transition::Enter{ exiting, .. } if !state.matches(exiting))
    })
    .system()
    .config(|(_, _, s)| *s = Some(state))
    .chain(should_run_adapter::<T>.system())
    .after(DriverLabel::<T>(PhantomData))
}

fn on_exit<T: Component + Clone>(state: PatternLiteral<T>) -> RunCriteriaDescriptor {
    (|current: Res<State<T>>,
      scratch: Res<StateScratchSpace<T>>,
      state: Required<PatternLiteral<T>>| {
        state.matches(&current.current)
            && matches!(&scratch.transition, Transition::Exit {entering, .. } if !state.matches(entering))
    })
    .system()
    .config(|(_, _, s)| *s = Some(state))
    .chain(should_run_adapter::<T>.system())
    .after(DriverLabel::<T>(PhantomData))
}

fn should_run_adapter<T: Component + Clone>(
    In(cmp_result): In<bool>,
    state: Res<StateScratchSpace<T>>,
) -> ShouldRun {
    if state.done {
        return ShouldRun::No;
    }
    if cmp_result {
        ShouldRun::YesAndCheckAgain
    } else {
        ShouldRun::NoAndCheckAgain
    }
}

mod test {
    use crate::{event::Events, prelude::*};

    use super::State;
    use super::*;

    #[derive(Copy, Clone)]
    enum SimpleState {
        Initial,
        A,
        B,
        C,
        D(bool),
    }

    #[test]
    fn simple_state() {
        let mut world = World::new();
        world.insert_resource({
            let mut events = Events::<StateChange<SimpleState>>::default();
            events.send(StateChange {
                f: |_| SimpleState::A,
                silent: false,
            });
            events
        });
        world.insert_resource(State {
            current: SimpleState::Initial,
        });
        world.insert_resource(StateScratchSpace::<SimpleState> {
            _marker: PhantomData,
            prepare_for_exit: false,
            done: false,
            transition: Transition::None,
        });

        let mut stage = SystemStage::parallel();

        stage.add_system_run_criteria(make_state_driver::<SimpleState>());
        stage.add_system(
            (|| println!("Entering SimpleState::A"))
                .system()
                .with_run_criteria(on_enter(pl!(SimpleState::A))),
        );
        stage.add_system(
            (|| println!("Updating SimpleState::A"))
                .system()
                .with_run_criteria(on_update(pl!(SimpleState::A))),
        );
        stage.add_system(
            (|| println!("Exiting SimpleState::A"))
                .system()
                .with_run_criteria(on_exit(pl!(SimpleState::A))),
        );
        stage.add_system(
            (|| println!("Entering SimpleState::B"))
                .system()
                .with_run_criteria(on_enter(pl!(SimpleState::B))),
        );
        stage.add_system(
            (|mut er: EventWriter<StateChange<SimpleState>>| {
                println!("Updating SimpleState::B");
                er.send(StateChange {
                    f: |_| SimpleState::C,
                    silent: false,
                });
            })
            .system()
            .with_run_criteria(on_update(pl!(SimpleState::B))),
        );
        stage.add_system(
            (|| println!("Exiting SimpleState::B"))
                .system()
                .with_run_criteria(on_exit(pl!(SimpleState::B))),
        );
        stage.add_system(
            (|mut ew: EventWriter<StateChange<SimpleState>>| {
                println!("Entering SimpleState::C");
                ew.send(StateChange {
                    f: |_| SimpleState::D(false),
                    silent: false,
                })
            })
            .system()
            .with_run_criteria(on_enter(pl!(SimpleState::C))),
        );
        stage.add_system(
            (|| println!("Updating SimpleState::C"))
                .system()
                .with_run_criteria(on_update(pl!(SimpleState::C))),
        );
        stage.add_system(
            (|| println!("Exiting SimpleState::C"))
                .system()
                .with_run_criteria(on_exit(pl!(SimpleState::C))),
        );
        stage.add_system(
            (|| println!("Entering SimpleState::D"))
                .system()
                .with_run_criteria(on_enter(pl!(SimpleState::D(_)))),
        );
        stage.add_system_set(
            SystemSet::new()
                .with_run_criteria(on_update(pl!(SimpleState::D(false))))
                .with_system(
                    (|| println!("Updating SimpleState::D"))
                        .system()
                        .before("ft"),
                )
                .with_system(
                    (|mut acc: Local<usize>, mut ew: EventWriter<StateChange<SimpleState>>| {
                        const DT: usize = 3;
                        *acc += 1;
                        if *acc >= DT {
                            *acc -= DT;
                            ew.send_batch([
                                StateChange {
                                    f: |_| SimpleState::D(true),
                                    silent: false,
                                },
                                StateChange {
                                    f: |_| SimpleState::D(false),
                                    silent: true,
                                },
                            ])
                        }
                    })
                    .system()
                    .label("ft"),
                ),
        );
        stage.add_system(
            (|| println!("Fixed Updating SimpleState::D"))
                .system()
                .with_run_criteria(on_update(pl!(SimpleState::D(true)))),
        );
        stage.add_system(
            (|| println!("Exiting SimpleState::D"))
                .system()
                .with_run_criteria(on_exit(pl!(SimpleState::D(_)))),
        );
        stage.run(&mut world);
        dbg!("first run done!");
        stage.run(&mut world);
        dbg!("second run done!");
        world
            .get_resource_mut::<Events<StateChange<SimpleState>>>()
            .unwrap()
            .send(StateChange {
                f: |_| SimpleState::B,
                silent: false,
            });
        stage.run(&mut world);
        dbg!("third run done!");
        world
            .get_resource_mut::<Events<StateChange<SimpleState>>>()
            .unwrap()
            .send(StateChange {
                f: |_| SimpleState::D(false),
                silent: false,
            });
        println!("start many runs");
        for i in 4..14 {
            stage.run(&mut world);
            println!("{}th run done", i)
        }
    }
}

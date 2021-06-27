use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use super::{RunCriteriaDescriptor, RunCriteriaDescriptorCoercion, ShouldRun};
use crate::schedule::label::RunCriteriaLabel;
use crate::{
    component::Component,
    prelude::{EventReader, In, IntoChainSystem, IntoSystem, Res, ResMut},
    system::Required,
};

#[derive(Clone, Copy)]
pub struct PatternLiteral<T>(pub fn(&T) -> bool, pub &'static str);
impl<T> PartialEq for PatternLiteral<T> {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl<T> Debug for PatternLiteral<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

impl<T> PatternLiteral<T> {
    fn matches(&self, t: &T) -> bool {
        self.0(t)
    }
}

pub struct State<T: Component + Clone> {
    // only None on startup
    current: Option<T>,
}

impl<T: Component + Clone> State<T> {
    pub fn new(initial: T) -> (Self, StateScratchSpace<T>) {
        (
            Self { current: None },
            StateScratchSpace {
                _marker: PhantomData,
                prepare_for_exit: false,
                done: false,
                transition: Transition::InitializeRequest { initial },
            },
        )
    }
}
#[derive(Clone, Copy)]
pub struct StateChange<T: Component + Clone> {
    pub v: T,
    pub update_same_frame: bool,
}
impl<T: Component + Clone> StateChange<T> {
    pub fn to(v: T) -> Self {
        Self {
            v,
            update_same_frame: false,
        }
    }
}

pub struct StateScratchSpace<T: Component + Clone> {
    _marker: PhantomData<T>,
    prepare_for_exit: bool,
    done: bool,
    transition: Transition<T>,
}

enum Transition<T> {
    None,
    Enter {
        exiting: T,
        update_same_frame: bool,
    },
    Exit {
        entering: T,
        update_same_frame: bool,
    },
    InitializeRequest {
        initial: T,
    },
    Initialize {
        initial: T,
    },
}

impl<T> Transition<T> {
    fn take(&mut self) -> Self {
        std::mem::replace(self, Transition::None)
    }
}

pub struct DriverLabel<T: Component>(PhantomData<T>);
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

impl<T: Component + Clone> State<T> {
    pub fn get_driver() -> RunCriteriaDescriptor {
        state_driver::<T>
            .system()
            .label(DriverLabel::<T>(PhantomData))
    }

    pub fn on_update(state: PatternLiteral<T>) -> RunCriteriaDescriptor {
        (|current: Res<State<T>>,
          scratch: Res<StateScratchSpace<T>>,
          state: Required<PatternLiteral<T>>| {
            if current.current.is_none() {
                return false;
            }

            state.matches(&current.current.as_ref().unwrap())
                && matches!(&scratch.transition, Transition::None)
        })
        .system()
        .config(|(_, _, s)| *s = Some(state))
        .chain(should_run_adapter::<T>.system())
        .after(DriverLabel::<T>(PhantomData))
    }

    pub fn on_enter(state: PatternLiteral<T>) -> RunCriteriaDescriptor {
        (|current: Res<State<T>>,
          scratch: Res<StateScratchSpace<T>>,
          state: Required<PatternLiteral<T>>| {
            if current.current.is_none() {
                return matches!(&scratch.transition, Transition::Initialize { initial } if state.matches(initial));
            }
            state.matches(&current.current.as_ref().unwrap())
                && matches!(&scratch.transition, Transition::Enter{ exiting, .. } if !state.matches(exiting))
        })
        .system()
        .config(|(_, _, s)| *s = Some(state))
        .chain(should_run_adapter::<T>.system())
        .after(DriverLabel::<T>(PhantomData))
    }

    pub fn on_exit(state: PatternLiteral<T>) -> RunCriteriaDescriptor {
        (|current: Res<State<T>>,
          scratch: Res<StateScratchSpace<T>>,
          state: Required<PatternLiteral<T>>| {
            if current.current.is_none() {
                return false;
            }
            state.matches(&current.current.as_ref().unwrap())
                && matches!(&scratch.transition, Transition::Exit {entering, .. } if !state.matches(entering))
        })
        .system()
        .config(|(_, _, s)| *s = Some(state))
        .chain(should_run_adapter::<T>.system())
        .after(DriverLabel::<T>(PhantomData))
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
                    entering: next.v.clone(),
                    update_same_frame: next.update_same_frame,
                }
            } else if scratch.done {
                scratch.done = false;
                return ShouldRun::No;
            } else if scratch.prepare_for_exit {
                scratch.prepare_for_exit = false;
                scratch.done = true;
            } else {
                scratch.prepare_for_exit = true;
            }
        }
        Transition::Enter {
            update_same_frame, ..
        } => {
            scratch.prepare_for_exit = true;
            if !update_same_frame {
                return state_driver(state, scratch, er);
            }
            scratch.transition = Transition::None;
        }
        Transition::Exit {
            entering,
            update_same_frame,
        } => {
            scratch.prepare_for_exit = false;
            scratch.transition = Transition::Enter {
                exiting: std::mem::replace(&mut state.current.as_mut().unwrap(), entering),
                update_same_frame,
            };
        }
        Transition::Initialize { initial } => {
            scratch.transition = Transition::None;
            state.current = Some(initial);
        }
        Transition::InitializeRequest { initial } => {
            scratch.transition = Transition::Initialize { initial };
        }
    }

    ShouldRun::NoAndCheckAgain
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

#[cfg(test)]
mod test {
    use crate::{event::Events, prelude::*};

    use super::*;

    #[derive(Copy, Clone)]
    enum SimpleState {
        A,
        B,
        C,
        D(bool),
    }

    impl SimpleState {
        const IN_A: PatternLiteral<Self> = pattern_literal!(Self::A);
        const IN_B: PatternLiteral<Self> = pattern_literal!(Self::B);
        const IN_C: PatternLiteral<Self> = pattern_literal!(Self::C);
        const IN_D: PatternLiteral<Self> = pattern_literal!(Self::D(false));
        const ANY_D: PatternLiteral<Self> = pattern_literal!(Self::D(_));
        const IN_D_FT: PatternLiteral<Self> = pattern_literal!(Self::D(true));
    }

    #[test]
    fn simple_state() {
        let mut world = World::new();
        world.insert_resource(Events::<StateChange<SimpleState>>::default());
        let (state, scratch) = State::new(SimpleState::A);
        world.insert_resource(state);
        world.insert_resource(scratch);

        let mut stage = SystemStage::parallel();

        stage.add_system_run_criteria(State::<SimpleState>::get_driver());
        stage.add_system(
            (|| println!("Entering SimpleState::A"))
                .system()
                .with_run_criteria(State::on_enter(SimpleState::IN_A)),
        );
        stage.add_system(
            (|| println!("Updating SimpleState::A"))
                .system()
                .with_run_criteria(State::on_update(SimpleState::IN_A)),
        );
        stage.add_system(
            (|| println!("Exiting SimpleState::A"))
                .system()
                .with_run_criteria(State::on_exit(SimpleState::IN_A)),
        );
        stage.add_system(
            (|| println!("Entering SimpleState::B"))
                .system()
                .with_run_criteria(State::on_enter(SimpleState::IN_B)),
        );
        stage.add_system(
            (|mut er: EventWriter<StateChange<SimpleState>>| {
                println!("Updating SimpleState::B");
                er.send(StateChange::to(SimpleState::C));
            })
            .system()
            .with_run_criteria(State::on_update(SimpleState::IN_B)),
        );
        stage.add_system(
            (|| println!("Exiting SimpleState::B"))
                .system()
                .with_run_criteria(State::on_exit(SimpleState::IN_B)),
        );
        stage.add_system(
            (|mut ew: EventWriter<StateChange<SimpleState>>| {
                println!("Entering SimpleState::C");
                ew.send(StateChange::to(SimpleState::D(false)))
            })
            .system()
            .with_run_criteria(State::on_enter(SimpleState::IN_C)),
        );
        stage.add_system(
            (|| println!("Updating SimpleState::C"))
                .system()
                .with_run_criteria(State::on_update(SimpleState::IN_C)),
        );
        stage.add_system(
            (|| println!("Exiting SimpleState::C"))
                .system()
                .with_run_criteria(State::on_exit(SimpleState::IN_C)),
        );
        stage.add_system(
            (|| println!("Entering SimpleState::D"))
                .system()
                .with_run_criteria(State::on_enter(SimpleState::ANY_D)),
        );
        stage.add_system_set(
            SystemSet::new()
                .with_run_criteria(State::on_update(SimpleState::IN_D))
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
                                    v: SimpleState::D(true),
                                    update_same_frame: true,
                                },
                                StateChange {
                                    v: SimpleState::D(false),
                                    update_same_frame: false,
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
                .with_run_criteria(State::on_update(SimpleState::IN_D_FT)),
        );
        stage.add_system(
            (|| println!("Exiting SimpleState::D"))
                .system()
                .with_run_criteria(State::on_exit(SimpleState::ANY_D)),
        );
        stage.run(&mut world);
        dbg!("first run done!");
        stage.run(&mut world);
        dbg!("second run done!");
        world
            .get_resource_mut::<Events<StateChange<SimpleState>>>()
            .unwrap()
            .send(StateChange::to(SimpleState::B));
        stage.run(&mut world);
        dbg!("third run done!");
        world
            .get_resource_mut::<Events<StateChange<SimpleState>>>()
            .unwrap()
            .send(StateChange::to(SimpleState::D(false)));
        println!("start many runs");
        for i in 4..14 {
            stage.run(&mut world);
            println!("{}th run done", i)
        }
    }
}

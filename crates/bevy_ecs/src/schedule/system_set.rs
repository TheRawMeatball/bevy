use crate::{RunCriteria, ShouldRun, System, SystemDescriptor};

pub struct SystemSet {
    pub(crate) run_criteria: RunCriteria,
    pub(crate) descriptors: Vec<SystemDescriptor>,
    pub(crate) children: Vec<SystemSet>,
}

impl Default for SystemSet {
    fn default() -> SystemSet {
        SystemSet {
            run_criteria: Default::default(),
            descriptors: vec![],
            children: vec![],
        }
    }
}

impl SystemSet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_run_criteria<S: System<In = (), Out = ShouldRun>>(mut self, system: S) -> Self {
        self.add_run_criteria(system);
        self
    }

    pub fn add_run_criteria<S: System<In = (), Out = ShouldRun>>(
        &mut self,
        system: S,
    ) -> &mut Self {
        self.run_criteria.set(Box::new(system));
        self
    }

    pub fn with_system(mut self, system: impl Into<SystemDescriptor>) -> Self {
        self.add_system(system);
        self
    }

    pub fn add_system(&mut self, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.descriptors.push(system.into());
        self
    }

    pub fn add_child(&mut self, system_set: SystemSet) -> &mut Self {
        self.children.push(system_set);
        self
    }
}

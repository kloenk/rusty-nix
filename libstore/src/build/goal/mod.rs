use log::*;
use std::rc::{Rc, Weak};

use super::worker::Worker;

pub mod derivation;

#[derive(Debug)]
pub enum ExitCode {
    EcBusy,
    EcSucces,
    EcFailed,
    EcNoSubstituters,
    EcIncompleteClosure,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    Forbidden,
    Misc,
    Transient,
    Interrupted,
}

pub type Goals = Vec<Rc<dyn Goal>>; // TODO: C++ uses set<Goal, `CompareGoalPtrs`>
pub type WeakGoals = Vec<Weak<dyn Goal>>;

pub trait Goal {
    fn key(&self) -> String;

    fn start_work(&mut self) -> Result<(), crate::error::BuildError>; // TODO: async?
}

/*pub struct Goal {
    /// Whether the goal is finished
    pub exit_code: ExitCode,

    /// Backlink to the worker
    pub worker: Rc<Worker>,

    /// Goals that this goal is waiting for.
    pub waitees: Goals,

    /// Goals waiting for this one to finish.  Must use weak pointers here to prevent cycles.
    pub waiters: WeakGoals,

    /// Number of goals we are/were waiting for that have failed.
    pub nr_failed: usize,

    /// Number of substitution goals we are/were waiting for that
    /// failed because there are no substituters.
    pub nr_no_substituters: usize,

    /// Number of substitution goals we are/were waiting for that
    /// failed because othey had unsubstitutable references.
    pub nr_incomplete_closure: usize,

    /// Name of this goal for debugging purposes.
    pub name: String,

    /// Exception containing an error message, if any.
    pub ex: Option<Error>,
}*/

/*impl Goal {
    pub fn new(worker: Rc<Worker>) -> Self {
        Self {
            worker,
            exit_code: ExitCode::EcBusy,
            nr_failed: 0,
            nr_incomplete_closure: 0,
            nr_no_substituters: 0,
            ex: None,
            name: String::new(),
            waitees: Vec::new(),
            waiters: Vec::new(),
        }
    }

}

impl Drop for Goal {
    fn drop(&mut self) {
       trace!("goal destroyed");
    }
}*/

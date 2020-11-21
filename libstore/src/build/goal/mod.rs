use log::*;
//use std::rc::{Rc, Weak};
use std::sync::{Arc, Weak};

use super::worker::Worker;
use crate::store::Store;

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

pub type Goals = Vec<Arc<dyn Goal>>; // TODO: C++ uses set<Goal, `CompareGoalPtrs`>
pub type WeakGoals = Vec<Weak<dyn Goal>>;

pub trait Goal {
    ///  Ensure that derivations get built in order of their name,
    ///  i.e. a derivation named "aardvark" always comes before
    ///  "baboon". And substitution goals always happen before
    ///  derivation goals (due to "b$").
    fn key(&self, store: &dyn Store) -> String;

    fn key_nostore(&self) -> String;

    fn start_work(&self) -> Result<(), crate::error::BuildError>; // TODO: async?
}

use std::fmt;
impl fmt::Debug for dyn Goal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Goal {{ {} }}", self.key_nostore())
    }
}

impl PartialEq for dyn Goal {
    fn eq(&self, other: &dyn Goal) -> bool {
        self.key_nostore() == other.key_nostore()
    }
}

impl PartialOrd for dyn Goal {
    fn partial_cmp(&self, other: &dyn Goal) -> Option<std::cmp::Ordering> {
        self.key_nostore().partial_cmp(&other.key_nostore())
    }
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

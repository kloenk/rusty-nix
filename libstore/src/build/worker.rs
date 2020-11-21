use crate::error::BuildError;
use crate::store::BuildStore;

use std::boxed::Box;
use std::sync::{Arc, Mutex, Weak};

use super::goal::Goal;

use crate::store::path::StorePath;

pub struct Worker {
    store: Box<dyn BuildStore>,

    /// the top-level goals of the worker.
    top_goals: Mutex<Vec<Arc<dyn super::goal::Goal>>>,

    /// Goals that are ready to do some work.
    awake: Mutex<Vec<Weak<dyn super::goal::Goal>>>,

    /*/* Note: the worker should only have strong pointers to the
       top-level goals. */

    /* The top-level goals of the worker. */
    Goals topGoals;

    /* Goals that are ready to do some work. */
    WeakGoals awake;

    /* Goals waiting for a build slot. */
    WeakGoals wantingToBuild;

    /* Child processes currently running. */
    std::list<Child> children;

    /* Maps used to prevent multiple instantiations of a goal for the
       same derivation / path. */
    WeakGoalMap derivationGoals;
    WeakGoalMap substitutionGoals;

    /* Goals waiting for busy paths to be unlocked. */
    WeakGoals waitingForAnyGoal;

    /* Goals sleeping for a few seconds (polling a lock). */
    WeakGoals waitingForAWhile;

    /* Cache for pathContentsGood(). */
    std::map<StorePath, bool> pathContentsGoodCache;*/
    /// Number of build slots occupied.  This includes local builds and
    /// substitutions but not remote builds via the build hook.
    nr_local_builds: usize,

    ///  Last time the goals in `waitingForAWhile' where woken up.
    last_woken_up: std::time::SystemTime,
    // public:
    /*
    const Activity act;
    const Activity actDerivations;
    const Activity actSubstitutions;

    /* Set if at least one derivation had a BuildError (i.e. permanent
       failure). */
    bool permanentFailure;

    /* Set if at least one derivation had a timeout. */
    bool timedOut;

    /* Set if at least one derivation fails with a hash mismatch. */
    bool hashMismatch;

    /* Set if at least one derivation is not deterministic in check mode. */
    bool checkMismatch;

    LocalStore & store;

    std::unique_ptr<HookInstance> hook;

    uint64_t expectedBuilds = 0;
    uint64_t doneBuilds = 0;
    uint64_t failedBuilds = 0;
    uint64_t runningBuilds = 0;

    uint64_t expectedSubstitutions = 0;
    uint64_t doneSubstitutions = 0;
    uint64_t failedSubstitutions = 0;
    uint64_t runningSubstitutions = 0;
    uint64_t expectedDownloadSize = 0;
    uint64_t doneDownloadSize = 0;
    uint64_t expectedNarSize = 0;
    uint64_t doneNarSize = 0;

    /* Whether to ask the build hook if it can build a derivation. If
       it answers with "decline-permanently", we don't try again. */
    bool tryBuildHook = true;

    Worker(LocalStore & store);
    ~Worker();

    /* Make a goal (with caching). */
    GoalPtr makeDerivationGoal(const StorePath & drvPath, const StringSet & wantedOutputs, BuildMode buildMode = bmNormal);
    std::shared_ptr<DerivationGoal> makeBasicDerivationGoal(const StorePath & drvPath,
        const BasicDerivation & drv, BuildMode buildMode = bmNormal);
    GoalPtr makeSubstitutionGoal(const StorePath & storePath, RepairFlag repair = NoRepair);

    /* Remove a dead goal. */
    void removeGoal(GoalPtr goal);

    /* Wake up a goal (i.e., there is something for it to do). */
    void wakeUp(GoalPtr goal);

    /* Return the number of local build and substitution processes
       currently running (but not remote builds via the build
       hook). */
    unsigned int getNrLocalBuilds();

    /* Registers a running child process.  `inBuildSlot' means that
       the process counts towards the jobs limit. */
    void childStarted(GoalPtr goal, const set<int> & fds,
        bool inBuildSlot, bool respectTimeouts);

    /* Unregisters a running child process.  `wakeSleepers' should be
       false if there is no sense in waking up goals that are sleeping
       because they can't run yet (e.g., there is no free build slot,
       or the hook would still say `postpone'). */
    void childTerminated(Goal * goal, bool wakeSleepers = true);

    /* Put `goal' to sleep until a build slot becomes available (which
       might be right away). */
    void waitForBuildSlot(GoalPtr goal);

    /* Wait for any goal to finish.  Pretty indiscriminate way to
       wait for some resource that some other goal is holding. */
    void waitForAnyGoal(GoalPtr goal);

    /* Wait for a few seconds and then retry this goal.  Used when
       waiting for a lock held by another process.  This kind of
       polling is inefficient, but POSIX doesn't really provide a way
       to wait for multiple locks in the main select() loop. */
    void waitForAWhile(GoalPtr goal);

    /* Loop until the specified top-level goals have finished. */
    void run(const Goals & topGoals);

    /* Wait for input to become available. */
    void waitForInput();

    unsigned int exitStatus();

    /* Check whether the given valid path exists and has the right
       contents. */
    bool pathContentsGood(const StorePath & path);

    void markContentsGood(const StorePath & path);

    void updateProgress()
    {
        actDerivations.progress(doneBuilds, expectedBuilds + doneBuilds, runningBuilds, failedBuilds);
        actSubstitutions.progress(doneSubstitutions, expectedSubstitutions + doneSubstitutions, runningSubstitutions, failedSubstitutions);
        act.setExpected(actFileTransfer, expectedDownloadSize + doneDownloadSize);
        act.setExpected(actCopyPath, expectedNarSize + doneNarSize);
    }
    */
}

impl Worker {
    pub fn new(store: Box<dyn BuildStore>) -> Arc<Self> {
        Arc::new(Self {
            store,
            last_woken_up: std::time::SystemTime::now(),
            nr_local_builds: 0,
            awake: Mutex::new(Vec::new()),
            top_goals: Mutex::new(Vec::new()),
        })
    }

    pub fn get_nr_local_builds(self: &Arc<Self>) -> usize {
        self.nr_local_builds
    }

    pub async fn run(
        self: &Arc<Self>,
        goals: Vec<Arc<dyn super::goal::Goal>>,
    ) -> Result<(), BuildError> {
        let mut goals = goals;
        goals.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut top_goals = self.top_goals.lock().unwrap();
        *top_goals = goals;
        drop(top_goals); // others need acces

        log::debug!("entered goal loop");

        loop {
            let mut do_inner = self.do_inner();

            while do_inner {
                let mut awake2 = Vec::new();
                let mut awake = self.awake.lock().unwrap();
                for g in awake.drain(..) {
                    awake2.push(g)
                }
                drop(awake);

                for g in awake2.drain(..) {
                    if let Some(Err(e)) = g.upgrade().map(|v| v.start_work()) {
                        return Err(e);
                    }
                    if self.top_goals.lock().unwrap().is_empty() {
                        break;
                    };
                }

                do_inner = self.do_inner();
            }

            //self.store.auto_gc(false)?;
        }
        unimplemented!()
    }

    fn do_inner(self: &Arc<Self>) -> bool {
        let awake = self.awake.lock().unwrap().is_empty();
        let top_goals = self.top_goals.lock().unwrap().is_empty();
        !awake && !top_goals
    }

    pub async fn make_derivation_goal(
        self: &Arc<Self>,
        drv_path: StorePath,
        wanted_outputs: Vec<String>,
        build_mode: BuildMode,
    ) -> Result<Arc<dyn Goal>, BuildError> {
        let store = self.store.box_clone();
        let parsed_drv = crate::build::derivation::Derivation::from_path(&drv_path, &(*store))
            .await
            .unwrap();
        let parsed_drv =
            crate::build::derivation::ParsedDerivation::new(drv_path.clone(), parsed_drv).unwrap();
        let parsed_drv = Arc::new(parsed_drv);

        let settings = crate::CONFIG.read().unwrap();

        let ret = super::goal::derivation::DerivationGoal {
            build_mode,
            wanted_outputs,
            drv_path,
            parsed_drv,
            build_user: None,
            use_derivation: false,
            chroot_root_dir: std::path::PathBuf::new(),
            cur_round: 0,
            current_hook_line: String::new(),
            current_log_line: String::new(),
            current_log_line_pos: 0,
            env: std::collections::HashMap::new(), // TODO
            ex: None,
            exit_code: super::goal::ExitCode::EcBusy,
            fixed_output: false,
            log_size: 0,
            log_tail: Vec::new(),
            machine_name: String::new(),
            missing_paths: Vec::new(),
            name: String::new(), // TODO?
            needs_restart: false,
            nr_failed: 0,
            nr_incomplete_closure: 0,
            nr_no_substituters: 0,
            nr_rounds: 0,
            pid: 0,
            private_network: true,
            retry_substitution: false,
            sandbox_gid: 0,
            sandbox_uid: 0,
            tmp_dir: std::path::PathBuf::new(),
            tmp_dir_in_sandbox: std::path::PathBuf::from(&settings.sandbox_build_dir),
            use_chroot: false, // TODO: settings,
            valid_paths: Vec::new(),
            waitees: Vec::new(),
            waiters: Vec::new(),
            worker: self.clone(),
        };

        let ret = Arc::new(ret) as Arc<dyn Goal>;

        self.wake_up(Arc::downgrade(&ret))?;

        Ok(ret)
    }

    fn wake_up(self: &Arc<Self>, goal: Weak<dyn Goal>) -> Result<(), BuildError> {
        let mut awake = self.awake.lock().unwrap();
        (*awake).push(goal);
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BuildMode {
    None,
}

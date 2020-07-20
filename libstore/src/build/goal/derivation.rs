use log::*;
use std::collections::HashMap;

use super::*;
use crate::store::path::{StorePath, StorePaths};

pub struct DerivationGoal {
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

    // -------------------------------------- //
    /// Whether to use an on-disk .drv file.
    pub use_derivation: bool,

    /// The Path of the Derivation
    pub drv_path: StorePath, // TODO: option? String?

    /// The specific outputs that we need to build.  Empty means all of them.
    pub wanted_outputs: Vec<String>,

    /// Whether additional ouptuts have been added
    pub needs_restart: bool,

    /// Whether to retry substituting the outputs after building the inputs.
    pub retry_substitution: bool,

    /*/* The derivation stored at drvPath. */
    std::unique_ptr<BasicDerivation> drv;

    std::unique_ptr<ParsedDerivation> parsedDrv;

    /* The remainder is state held during the build. */

    /* Locks on the output paths. */
    PathLocks outputLocks;
        /* All input paths (that is, the union of FS closures of the
       immediate input paths). */
    StorePathSet inputPaths;*/
    /// Outputs that are already valid.  If we're repairing, these are
    /// the outputs that are valid *and* not corrupt.
    pub valid_paths: StorePaths, // TODO: String?

    /// Outputs that are corrupt or not valid.
    pub missing_paths: StorePaths,

    /// User selected for running the builder.
    pub build_user: crate::build::user::UserLock,

    /// The process ID of the builder.
    pub pid: libc::pid_t,

    /// The temporary directory.
    pub tmp_dir: std::path::PathBuf,

    /// The path of the temporary directory in the sandbox.
    pub tmp_dir_in_sandbox: std::path::PathBuf,

    /* File descriptor for the log file.
    AutoCloseFD fdLogFile;
    std::shared_ptr<BufferedSink> logFileSink, logSink;*/
    /// Number of bytes received from the builder's stdout/stderr.
    pub log_size: usize,

    /// The most recent log lines.
    pub log_tail: Vec<String>,

    /// Current log line
    pub current_log_line: String,

    /// To handle carriage return
    pub current_log_line_pos: usize,

    /// TODOpub : ??
    pub current_hook_line: String,

    /* Pipe for the builder's standard output/error.
    Pipe builderOut;

    /* Pipe for synchronising updates to the builder namespaces. */
    Pipe userNamespaceSync;*/

    /* The mount namespace of the builder, used to add additional
       paths to the sandbox as a result of recursive Nix calls.
    AutoCloseFD sandboxMountNamespace;*/

    /* The build hook.
    std::unique_ptr<HookInstance> hook;*/
    /// Whether we're currently doing a chroot build.
    pub use_chroot: bool,

    /// Path of the Chroot dir
    pub chroot_root_dir: std::path::PathBuf,

    /* RAII object to delete the chroot directory.
    std::shared_ptr<AutoDelete> autoDelChroot;*/
    /// Whether this is a fixed-output derivation.
    pub fixed_output: bool,

    /// Whether to run the build in a private network namespace.
    pub private_network: bool,

    /*
    typedef void (DerivationGoal::*GoalState)();
    GoalState state;

    /* Stuff we need to pass to initChild(). */
    struct ChrootPath {
        Path source;
        bool optional;
        ChrootPath(Path source = "", bool optional = false)
            : source(source), optional(optional)
        { }
    };
    typedef map<Path, ChrootPath> DirsInChroot; // maps target path to source path
    DirsInChroot dirsInChroot;

    typedef map<string, string> Environment;
    Environment env;
    */
    /// Environment
    pub env: HashMap<String, String>,

    #[cfg(target_os = "darwin")]
    /// SandboxProfile used on darwin
    pub additional_sandbox_profile: String,
    /*

    #if __APPLE__
        typedef string SandboxProfile;
        SandboxProfile additionalSandboxProfile;
    #endif

        /* Hash rewriting. */
        StringMap inputRewrites, outputRewrites;
        typedef map<StorePath, StorePath> RedirectedOutputs;
        RedirectedOutputs redirectedOutputs;

        BuildMode buildMode;

        /* If we're repairing without a chroot, there may be outputs that
           are valid but corrupt.  So we redirect these outputs to
           temporary paths. */
        StorePathSet redirectedBadOutputs;

        BuildResult result;
        */
    /// The current round, if we're building multiple times
    pub cur_round: usize,

    /// number of rounds when building multiple times
    pub nr_rounds: usize,

    /*
    /* Path registration info from the previous round, if we're
       building multiple times. Since this contains the hash, it
       allows us to compare whether two rounds produced the same
       result. */
    std::map<Path, ValidPathInfo> prevInfos;
    */
    pub sandbox_uid: libc::uid_t,
    pub sandbox_gid: libc::gid_t,

    /// path to the home dir
    pub home_dir: &'static str,

    /*
    std::unique_ptr<MaintainCount<uint64_t>> mcExpectedBuilds, mcRunningBuilds;

    std::unique_ptr<Activity> act;

    /* Activity that denotes waiting for a lock. */
    std::unique_ptr<Activity> actLock;

    std::map<ActivityId, Activity> builderActivities;

    */
    /// The remote machine on which we're building
    pub machine_name: String,
    /*
    /* The recursive Nix daemon socket. */
    AutoCloseFD daemonSocket;

    /* The daemon main thread. */
    std::thread daemonThread;

    /* The daemon worker threads. */
    std::vector<std::thread> daemonWorkerThreads;

    /* Paths that were added via recursive Nix calls. */
    StorePathSet addedPaths;

        /* Recursive Nix calls are only allowed to build or realize paths
       in the original input closure or added via a recursive Nix call
       (so e.g. you can't do 'nix-store -r /nix/store/<bla>' where
       /nix/store/<bla> is some arbitrary path in a binary cache). */
    bool isAllowed(const StorePath & path)
    {
        return inputPaths.count(path) || addedPaths.count(path);
    }
    */
}

impl DerivationGoal {
    /*pub fn new(worker: Rc<Worker>) -> Self {
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
            use_derivation: false
        }
    }*/

    pub fn get_drv_path(&self) -> StorePath {
        self.drv_path.clone()
    }
}

impl Goal for DerivationGoal {
    fn key(&self) -> String {
        /* Ensure that derivations get built in order of their name,
         * i.e. a derivation named "aardvark" always comes before
         * "baboon". And substitution goals always happen before
         * derivation goals (due to "b$"). */
        //return "b$" + std::string(drvPath.name()) + "$" + worker.store.printStorePath(drvPath);
        format!(
            "b${}${}",
            self.drv_path.name(),
            self.drv_path.print_store_path()
        ) // TODO: first should only be the name
    }

    fn start_work(&mut self) -> Result<(), crate::error::BuildError> {
        unimplemented!()
        /*     /* Right platform? */
            if (!parsedDrv->canBuildLocally())
                throw Error("a '%s' with features {%s} is required to build '%s', but I am a '%s' with features {%s}",
                    drv->platform,
                    concatStringsSep(", ", parsedDrv->getRequiredSystemFeatures()),
                    worker.store.printStorePath(drvPath),
                    settings.thisSystem,
                    concatStringsSep<StringSet>(", ", settings.systemFeatures));

            if (drv->isBuiltin())
                preloadNSS();

        #if __APPLE__
            additionalSandboxProfile = parsedDrv->getStringAttr("__sandboxProfile").value_or("");
        #endif

            /* Are we doing a chroot build? */
            {
                auto noChroot = parsedDrv->getBoolAttr("__noChroot");
                if (settings.sandboxMode == smEnabled) {
                    if (noChroot)
                        throw Error("derivation '%s' has '__noChroot' set, "
                            "but that's not allowed when 'sandbox' is 'true'", worker.store.printStorePath(drvPath));
        #if __APPLE__
                    if (additionalSandboxProfile != "")
                        throw Error("derivation '%s' specifies a sandbox profile, "
                            "but this is only allowed when 'sandbox' is 'relaxed'", worker.store.printStorePath(drvPath));
        #endif
                    useChroot = true;
                }
                else if (settings.sandboxMode == smDisabled)
                    useChroot = false;
                else if (settings.sandboxMode == smRelaxed)
                    useChroot = !fixedOutput && !noChroot;
            }

            if (worker.store.storeDir != worker.store.realStoreDir) {
                #if __linux__
                    useChroot = true;
                #else
                    throw Error("building using a diverted store is not supported on this platform");
                #endif
            }

            /* Create a temporary directory where the build will take
               place. */
            tmpDir = createTempDir("", "nix-build-" + std::string(drvPath.name()), false, false, 0700);

            chownToBuilder(tmpDir);

            /* Substitute output placeholders with the actual output paths. */
            for (auto & output : drv->outputs)
                inputRewrites[hashPlaceholder(output.first)] = worker.store.printStorePath(output.second.path);

            /* Construct the environment passed to the builder. */
            initEnv();

            writeStructuredAttrs();

            /* Handle exportReferencesGraph(), if set. */
            if (!parsedDrv->getStructuredAttrs()) {
                /* The `exportReferencesGraph' feature allows the references graph
                   to be passed to a builder.  This attribute should be a list of
                   pairs [name1 path1 name2 path2 ...].  The references graph of
                   each `pathN' will be stored in a text file `nameN' in the
                   temporary build directory.  The text files have the format used
                   by `nix-store --register-validity'.  However, the deriver
                   fields are left empty. */
                string s = get(drv->env, "exportReferencesGraph").value_or("");
                Strings ss = tokenizeString<Strings>(s);
                if (ss.size() % 2 != 0)
                    throw BuildError("odd number of tokens in 'exportReferencesGraph': '%1%'", s);
                for (Strings::iterator i = ss.begin(); i != ss.end(); ) {
                    string fileName = *i++;
                    static std::regex regex("[A-Za-z_][A-Za-z0-9_.-]*");
                    if (!std::regex_match(fileName, regex))
                        throw Error("invalid file name '%s' in 'exportReferencesGraph'", fileName);

                    auto storePathS = *i++;
                    if (!worker.store.isInStore(storePathS))
                        throw BuildError("'exportReferencesGraph' contains a non-store path '%1%'", storePathS);
                    auto storePath = worker.store.parseStorePath(worker.store.toStorePath(storePathS));

                    /* Write closure info to <fileName>. */
                    writeFile(tmpDir + "/" + fileName,
                        worker.store.makeValidityRegistration(
                            exportReferences({storePath}), false, false));
                }
            }

            if (useChroot) {

                /* Allow a user-configurable set of directories from the
                   host file system. */
                PathSet dirs = settings.sandboxPaths;
                PathSet dirs2 = settings.extraSandboxPaths;
                dirs.insert(dirs2.begin(), dirs2.end());

                dirsInChroot.clear();

                for (auto i : dirs) {
                    if (i.empty()) continue;
                    bool optional = false;
                    if (i[i.size() - 1] == '?') {
                        optional = true;
                        i.pop_back();
                    }
                    size_t p = i.find('=');
                    if (p == string::npos)
                        dirsInChroot[i] = {i, optional};
                    else
                        dirsInChroot[string(i, 0, p)] = {string(i, p + 1), optional};
                }
                dirsInChroot[tmpDirInSandbox] = tmpDir;

                /* Add the closure of store paths to the chroot. */
                StorePathSet closure;
                for (auto & i : dirsInChroot)
                    try {
                        if (worker.store.isInStore(i.second.source))
                            worker.store.computeFSClosure(worker.store.parseStorePath(worker.store.toStorePath(i.second.source)), closure);
                    } catch (InvalidPath & e) {
                    } catch (Error & e) {
                        throw Error("while processing 'sandbox-paths': %s", e.what());
                    }
                for (auto & i : closure) {
                    auto p = worker.store.printStorePath(i);
                    dirsInChroot.insert_or_assign(p, p);
                }

                PathSet allowedPaths = settings.allowedImpureHostPrefixes;

                /* This works like the above, except on a per-derivation level */
                auto impurePaths = parsedDrv->getStringsAttr("__impureHostDeps").value_or(Strings());

                for (auto & i : impurePaths) {
                    bool found = false;
                    /* Note: we're not resolving symlinks here to prevent
                       giving a non-root user info about inaccessible
                       files. */
                    Path canonI = canonPath(i);
                    /* If only we had a trie to do this more efficiently :) luckily, these are generally going to be pretty small */
                    for (auto & a : allowedPaths) {
                        Path canonA = canonPath(a);
                        if (canonI == canonA || isInDir(canonI, canonA)) {
                            found = true;
                            break;
                        }
                    }
                    if (!found)
                        throw Error("derivation '%s' requested impure path '%s', but it was not in allowed-impure-host-deps",
                            worker.store.printStorePath(drvPath), i);

                    dirsInChroot[i] = i;
                }

        #if __linux__
                /* Create a temporary directory in which we set up the chroot
                   environment using bind-mounts.  We put it in the Nix store
                   to ensure that we can create hard-links to non-directory
                   inputs in the fake Nix store in the chroot (see below). */
                chrootRootDir = worker.store.Store::toRealPath(drvPath) + ".chroot";
                deletePath(chrootRootDir);

                /* Clean up the chroot directory automatically. */
                autoDelChroot = std::make_shared<AutoDelete>(chrootRootDir);

                printMsg(lvlChatty, format("setting up chroot environment in '%1%'") % chrootRootDir);

                if (mkdir(chrootRootDir.c_str(), 0750) == -1)
                    throw SysError("cannot create '%1%'", chrootRootDir);

                if (buildUser && chown(chrootRootDir.c_str(), 0, buildUser->getGID()) == -1)
                    throw SysError("cannot change ownership of '%1%'", chrootRootDir);

                /* Create a writable /tmp in the chroot.  Many builders need
                   this.  (Of course they should really respect $TMPDIR
                   instead.) */
                Path chrootTmpDir = chrootRootDir + "/tmp";
                createDirs(chrootTmpDir);
                chmod_(chrootTmpDir, 01777);

                /* Create a /etc/passwd with entries for the build user and the
                   nobody account.  The latter is kind of a hack to support
                   Samba-in-QEMU. */
                createDirs(chrootRootDir + "/etc");

                writeFile(chrootRootDir + "/etc/passwd", fmt(
                        "root:x:0:0:Nix build user:%3%:/noshell\n"
                        "nixbld:x:%1%:%2%:Nix build user:%3%:/noshell\n"
                        "nobody:x:65534:65534:Nobody:/:/noshell\n",
                        sandboxUid, sandboxGid, settings.sandboxBuildDir));

                /* Declare the build user's group so that programs get a consistent
                   view of the system (e.g., "id -gn"). */
                writeFile(chrootRootDir + "/etc/group",
                    (format(
                        "root:x:0:\n"
                        "nixbld:!:%1%:\n"
                        "nogroup:x:65534:\n") % sandboxGid).str());

                /* Create /etc/hosts with localhost entry. */
                if (!fixedOutput)
                    writeFile(chrootRootDir + "/etc/hosts", "127.0.0.1 localhost\n::1 localhost\n");

                /* Make the closure of the inputs available in the chroot,
                   rather than the whole Nix store.  This prevents any access
                   to undeclared dependencies.  Directories are bind-mounted,
                   while other inputs are hard-linked (since only directories
                   can be bind-mounted).  !!! As an extra security
                   precaution, make the fake Nix store only writable by the
                   build user. */
                Path chrootStoreDir = chrootRootDir + worker.store.storeDir;
                createDirs(chrootStoreDir);
                chmod_(chrootStoreDir, 01775);

                if (buildUser && chown(chrootStoreDir.c_str(), 0, buildUser->getGID()) == -1)
                    throw SysError("cannot change ownership of '%1%'", chrootStoreDir);

                for (auto & i : inputPaths) {
                    auto p = worker.store.printStorePath(i);
                    Path r = worker.store.toRealPath(p);
                    struct stat st;
                    if (lstat(r.c_str(), &st))
                        throw SysError("getting attributes of path '%s'", p);
                    if (S_ISDIR(st.st_mode))
                        dirsInChroot.insert_or_assign(p, r);
                    else
                        linkOrCopy(r, chrootRootDir + p);
                }

                /* If we're repairing, checking or rebuilding part of a
                   multiple-outputs derivation, it's possible that we're
                   rebuilding a path that is in settings.dirsInChroot
                   (typically the dependencies of /bin/sh).  Throw them
                   out. */
                for (auto & i : drv->outputs)
                    dirsInChroot.erase(worker.store.printStorePath(i.second.path));

        #elif __APPLE__
                /* We don't really have any parent prep work to do (yet?)
                   All work happens in the child, instead. */
        #else
                throw Error("sandboxing builds is not supported on this platform");
        #endif
            }

            if (needsHashRewrite()) {

                if (pathExists(homeDir))
                    throw Error("home directory '%1%' exists; please remove it to assure purity of builds without sandboxing", homeDir);

                /* We're not doing a chroot build, but we have some valid
                   output paths.  Since we can't just overwrite or delete
                   them, we have to do hash rewriting: i.e. in the
                   environment/arguments passed to the build, we replace the
                   hashes of the valid outputs with unique dummy strings;
                   after the build, we discard the redirected outputs
                   corresponding to the valid outputs, and rewrite the
                   contents of the new outputs to replace the dummy strings
                   with the actual hashes. */
                if (validPaths.size() > 0)
                    for (auto & i : validPaths)
                        addHashRewrite(i);

                /* If we're repairing, then we don't want to delete the
                   corrupt outputs in advance.  So rewrite them as well. */
                if (buildMode == bmRepair)
                    for (auto & i : missingPaths)
                        if (worker.store.isValidPath(i) && pathExists(worker.store.printStorePath(i))) {
                            addHashRewrite(i);
                            redirectedBadOutputs.insert(i);
                        }
            }

            if (useChroot && settings.preBuildHook != "" && dynamic_cast<Derivation *>(drv.get())) {
                printMsg(lvlChatty, format("executing pre-build hook '%1%'")
                    % settings.preBuildHook);
                auto args = useChroot ? Strings({worker.store.printStorePath(drvPath), chrootRootDir}) :
                    Strings({ worker.store.printStorePath(drvPath) });
                enum BuildHookState {
                    stBegin,
                    stExtraChrootDirs
                };
                auto state = stBegin;
                auto lines = runProgram(settings.preBuildHook, false, args);
                auto lastPos = std::string::size_type{0};
                for (auto nlPos = lines.find('\n'); nlPos != string::npos;
                        nlPos = lines.find('\n', lastPos)) {
                    auto line = std::string{lines, lastPos, nlPos - lastPos};
                    lastPos = nlPos + 1;
                    if (state == stBegin) {
                        if (line == "extra-sandbox-paths" || line == "extra-chroot-dirs") {
                            state = stExtraChrootDirs;
                        } else {
                            throw Error("unknown pre-build hook command '%1%'", line);
                        }
                    } else if (state == stExtraChrootDirs) {
                        if (line == "") {
                            state = stBegin;
                        } else {
                            auto p = line.find('=');
                            if (p == string::npos)
                                dirsInChroot[line] = line;
                            else
                                dirsInChroot[string(line, 0, p)] = string(line, p + 1);
                        }
                    }
                }
            }

            /* Fire up a Nix daemon to process recursive Nix calls from the
               builder. */
            if (parsedDrv->getRequiredSystemFeatures().count("recursive-nix"))
                startDaemon();

            /* Run the builder. */
            printMsg(lvlChatty, "executing builder '%1%'", drv->builder);

            /* Create the log file. */
            Path logFile = openLogFile();

            /* Create a pipe to get the output of the builder. */
            //builderOut.create();

            builderOut.readSide = posix_openpt(O_RDWR | O_NOCTTY);
            if (!builderOut.readSide)
                throw SysError("opening pseudoterminal master");

            std::string slaveName(ptsname(builderOut.readSide.get()));

            if (buildUser) {
                if (chmod(slaveName.c_str(), 0600))
                    throw SysError("changing mode of pseudoterminal slave");

                if (chown(slaveName.c_str(), buildUser->getUID(), 0))
                    throw SysError("changing owner of pseudoterminal slave");
            }
        #if __APPLE__
            else {
                if (grantpt(builderOut.readSide.get()))
                    throw SysError("granting access to pseudoterminal slave");
            }
        #endif

            #if 0
            // Mount the pt in the sandbox so that the "tty" command works.
            // FIXME: this doesn't work with the new devpts in the sandbox.
            if (useChroot)
                dirsInChroot[slaveName] = {slaveName, false};
            #endif

            if (unlockpt(builderOut.readSide.get()))
                throw SysError("unlocking pseudoterminal");

            builderOut.writeSide = open(slaveName.c_str(), O_RDWR | O_NOCTTY);
            if (!builderOut.writeSide)
                throw SysError("opening pseudoterminal slave");

            // Put the pt into raw mode to prevent \n -> \r\n translation.
            struct termios term;
            if (tcgetattr(builderOut.writeSide.get(), &term))
                throw SysError("getting pseudoterminal attributes");

            cfmakeraw(&term);

            if (tcsetattr(builderOut.writeSide.get(), TCSANOW, &term))
                throw SysError("putting pseudoterminal into raw mode");

            result.startTime = time(0);

            /* Fork a child to build the package. */
            ProcessOptions options;

        #if __linux__
            if (useChroot) {
                /* Set up private namespaces for the build:

                   - The PID namespace causes the build to start as PID 1.
                     Processes outside of the chroot are not visible to those
                     on the inside, but processes inside the chroot are
                     visible from the outside (though with different PIDs).

                   - The private mount namespace ensures that all the bind
                     mounts we do will only show up in this process and its
                     children, and will disappear automatically when we're
                     done.

                   - The private network namespace ensures that the builder
                     cannot talk to the outside world (or vice versa).  It
                     only has a private loopback interface. (Fixed-output
                     derivations are not run in a private network namespace
                     to allow functions like fetchurl to work.)

                   - The IPC namespace prevents the builder from communicating
                     with outside processes using SysV IPC mechanisms (shared
                     memory, message queues, semaphores).  It also ensures
                     that all IPC objects are destroyed when the builder
                     exits.

                   - The UTS namespace ensures that builders see a hostname of
                     localhost rather than the actual hostname.

                   We use a helper process to do the clone() to work around
                   clone() being broken in multi-threaded programs due to
                   at-fork handlers not being run. Note that we use
                   CLONE_PARENT to ensure that the real builder is parented to
                   us.
                */

                if (!fixedOutput)
                    privateNetwork = true;

                userNamespaceSync.create();

                options.allowVfork = false;

                Pid helper = startProcess([&]() {

                    /* Drop additional groups here because we can't do it
                       after we've created the new user namespace.  FIXME:
                       this means that if we're not root in the parent
                       namespace, we can't drop additional groups; they will
                       be mapped to nogroup in the child namespace. There does
                       not seem to be a workaround for this. (But who can tell
                       from reading user_namespaces(7)?)
                       See also https://lwn.net/Articles/621612/. */
                    if (getuid() == 0 && setgroups(0, 0) == -1)
                        throw SysError("setgroups failed");

                    size_t stackSize = 1 * 1024 * 1024;
                    char * stack = (char *) mmap(0, stackSize,
                        PROT_WRITE | PROT_READ, MAP_PRIVATE | MAP_ANONYMOUS | MAP_STACK, -1, 0);
                    if (stack == MAP_FAILED) throw SysError("allocating stack");

                    int flags = CLONE_NEWUSER | CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWIPC | CLONE_NEWUTS | CLONE_PARENT | SIGCHLD;
                    if (privateNetwork)
                        flags |= CLONE_NEWNET;

                    pid_t child = clone(childEntry, stack + stackSize, flags, this);
                    if (child == -1 && errno == EINVAL) {
                        /* Fallback for Linux < 2.13 where CLONE_NEWPID and
                           CLONE_PARENT are not allowed together. */
                        flags &= ~CLONE_NEWPID;
                        child = clone(childEntry, stack + stackSize, flags, this);
                    }
                    if (child == -1 && (errno == EPERM || errno == EINVAL)) {
                        /* Some distros patch Linux to not allow unprivileged
                         * user namespaces. If we get EPERM or EINVAL, try
                         * without CLONE_NEWUSER and see if that works.
                         */
                        flags &= ~CLONE_NEWUSER;
                        child = clone(childEntry, stack + stackSize, flags, this);
                    }
                    /* Otherwise exit with EPERM so we can handle this in the
                       parent. This is only done when sandbox-fallback is set
                       to true (the default). */
                    if (child == -1 && (errno == EPERM || errno == EINVAL) && settings.sandboxFallback)
                        _exit(1);
                    if (child == -1) throw SysError("cloning builder process");

                    writeFull(builderOut.writeSide.get(), std::to_string(child) + "\n");
                    _exit(0);
                }, options);

                int res = helper.wait();
                if (res != 0 && settings.sandboxFallback) {
                    useChroot = false;
                    initTmpDir();
                    goto fallback;
                } else if (res != 0)
                    throw Error("unable to start build process");

                userNamespaceSync.readSide = -1;

                pid_t tmp;
                if (!string2Int<pid_t>(readLine(builderOut.readSide.get()), tmp)) abort();
                pid = tmp;

                /* Set the UID/GID mapping of the builder's user namespace
                   such that the sandbox user maps to the build user, or to
                   the calling user (if build users are disabled). */
                uid_t hostUid = buildUser ? buildUser->getUID() : getuid();
                uid_t hostGid = buildUser ? buildUser->getGID() : getgid();

                writeFile("/proc/" + std::to_string(pid) + "/uid_map",
                    (format("%d %d 1") % sandboxUid % hostUid).str());

                writeFile("/proc/" + std::to_string(pid) + "/setgroups", "deny");

                writeFile("/proc/" + std::to_string(pid) + "/gid_map",
                    (format("%d %d 1") % sandboxGid % hostGid).str());

                /* Save the mount namespace of the child. We have to do this
                   *before* the child does a chroot. */
                sandboxMountNamespace = open(fmt("/proc/%d/ns/mnt", (pid_t) pid).c_str(), O_RDONLY);
                if (sandboxMountNamespace.get() == -1)
                    throw SysError("getting sandbox mount namespace");

                /* Signal the builder that we've updated its user namespace. */
                writeFull(userNamespaceSync.writeSide.get(), "1");
                userNamespaceSync.writeSide = -1;

            } else
        #endif
            {
            fallback:
                options.allowVfork = !buildUser && !drv->isBuiltin();
                pid = startProcess([&]() {
                    runChild();
                }, options);
            }

            /* parent */
            pid.setSeparatePG(true);
            builderOut.writeSide = -1;
            worker.childStarted(shared_from_this(), {builderOut.readSide.get()}, true, true);

            /* Check if setting up the build environment failed. */
            while (true) {
                string msg = readLine(builderOut.readSide.get());
                if (string(msg, 0, 1) == "\1") {
                    if (msg.size() == 1) break;
                    throw Error(string(msg, 1));
                }
                debug(msg);
            }
        } */
    }
}

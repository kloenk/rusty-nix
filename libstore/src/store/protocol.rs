#[repr(u32)]
#[derive(Debug)]
pub enum WorkerOp {
    WopInvalidRequest = 0, // Invalid Request
    WopIsValidPath = 1,
    WopHasSubstitutes = 3,
    WopQueryPathHash = 4,   // obsolete
    WopQueryReferences = 5, // obsolete
    WopQueryReferrers = 6,
    WopAddToStore = 7,
    WopAddTextToStore = 8,
    WopBuildPaths = 9,
    WopEnsurePath = 10,
    WopAddTempRoot = 11,
    WopAddIndirectRoot = 12,
    WopSyncWithGC = 13,
    WopFindRoots = 14,
    WopExportPath = 16,   // obsolete
    WopQueryDeriver = 18, // obsolete
    WopSetOptions = 19,
    WopCollectGarbage = 20,
    WopQuerySubstitutablePathInfo = 21,
    WopQueryDerivationOutputs = 22,
    WopQueryAllValidPaths = 23,
    WopQueryFailedPaths = 24,
    WopClearFailedPaths = 25,
    WopQueryPathInfo = 26,
    WopImportPaths = 27,                // obsolete
    WopQueryDerivationOutputNames = 28, // obsolete
    WopQueryPathFromHashPart = 29,
    WopQuerySubstitutablePathInfos = 30,
    WopQueryValidPaths = 31,
    WopQuerySubstitutablePaths = 32,
    WopQueryValidDerivers = 33,
    WopOptimiseStore = 34,
    WopVerifyStore = 35,
    WopBuildDerivation = 36,
    WopAddSignatures = 37,
    WopNarFromPath = 38,
    WopAddToStoreNar = 39,
    WopQueryMissing = 40,
}

impl From<u32> for WorkerOp {
    fn from(num: u32) -> Self {
        use WorkerOp::*;
        match num {
            1 => WopIsValidPath,
            3 => WopHasSubstitutes,
            4 => WorkerOp::WopQueryPathHash,
            5 => WorkerOp::WopQueryReferences,
            6 => WorkerOp::WopQueryReferrers,
            7 => WorkerOp::WopAddToStore,
            8 => WorkerOp::WopAddTextToStore,
            9 => WorkerOp::WopBuildPaths,
            10 => WorkerOp::WopEnsurePath,
            11 => WorkerOp::WopAddTempRoot,
            12 => WorkerOp::WopAddIndirectRoot,
            13 => WorkerOp::WopSyncWithGC,
            14 => WorkerOp::WopFindRoots,
            16 => WorkerOp::WopExportPath,
            18 => WorkerOp::WopQueryDeriver,
            19 => WorkerOp::WopSetOptions,
            20 => WorkerOp::WopCollectGarbage,
            21 => WorkerOp::WopQuerySubstitutablePathInfo,
            22 => WorkerOp::WopQueryDerivationOutputs,
            23 => WorkerOp::WopQueryAllValidPaths,
            24 => WorkerOp::WopQueryFailedPaths,
            25 => WorkerOp::WopClearFailedPaths,
            26 => WorkerOp::WopQueryPathInfo,
            27 => WorkerOp::WopImportPaths,
            28 => WorkerOp::WopQueryDerivationOutputNames,
            29 => WorkerOp::WopQueryPathFromHashPart,
            30 => WorkerOp::WopQuerySubstitutablePathInfos,
            31 => WorkerOp::WopQueryValidPaths,
            32 => WorkerOp::WopQuerySubstitutablePaths,
            33 => WorkerOp::WopQueryValidDerivers,
            34 => WorkerOp::WopOptimiseStore,
            35 => WorkerOp::WopVerifyStore,
            36 => WorkerOp::WopBuildDerivation,
            37 => WorkerOp::WopAddSignatures,
            38 => WorkerOp::WopNarFromPath,
            39 => WorkerOp::WopAddToStoreNar,
            40 => WorkerOp::WopQueryMissing,

            _ => WorkerOp::WopInvalidRequest,
        }
    }
}

#[repr(u32)]
#[derive(Debug)]
pub enum Verbosity {
    LVLError = 0,
    LVLWarn = 1,
    LVLInfo = 2,
    LVLTalkative = 3,
    LVLChatty = 4,
    LVLDebug = 5,
    LVLVomit = 6,
}

impl From<u32> for Verbosity {
    fn from(v: u32) -> Self {
        match v {
            0 => Verbosity::LVLError,
            1 => Verbosity::LVLWarn,
            2 => Verbosity::LVLInfo,
            3 => Verbosity::LVLTalkative,
            4 => Verbosity::LVLChatty,
            5 => Verbosity::LVLDebug,
            6 => Verbosity::LVLVomit,
            _ => Verbosity::LVLError, // Will fallback to LVLError
        }
    }
}

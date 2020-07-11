#[derive(Debug)]
pub enum ExitCode {
    EcBusy,
    EcSucces,
    EcFailed,
    EcNoSubstituters,
    EcIncompleteClosure,
}

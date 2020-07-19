use super::{AsyncWrite, Box, EmptyResult, LocalFutureObj};

#[repr(u64)]
#[allow(non_camel_case_types)]
pub enum STDERR {
    NEXT = 0x6f6c6d67,
    READ = 0x64617461,
    WRITE = 0x64617416,
    LAST = 0x616c7473,
    ERROR = 0x63787470,
    START_ACTIVITY = 0x53545254,
    STOP_ACTIVITY = 0x53544f50,
    RESULT = 0x52534c54,
}

// TODO: should this implemen AsyncRead?
pub trait Logger: AsyncWrite {
    /// determinds if the logger can send data
    fn can_send(&self) -> bool;

    /// sets the can send variable
    fn set_can_send(&self, can: bool);

    /// add data onto the message queu
    fn enqueu(&self, msg: String);

    /// retriev all non send messages
    /// this must clear the message queue
    fn dequeu(&self) -> Vec<String>;

    fn start_work<'a>(&'a self) -> LocalFutureObj<'a, EmptyResult> {
        LocalFutureObj::new(Box::new(async move {
            self.set_can_send(true);

            for v in self.dequeu() {
                // TODO: something better?
                self.write(v.as_bytes()).await?;
            }
            Ok(())
        }))
    }

    fn stop_work<'a>(&'a self, state: WorkFinish) -> LocalFutureObj<'a, EmptyResult> {
        LocalFutureObj::new(Box::new(async move {
            self.set_can_send(false);

            match state {
                WorkFinish::Error(msg, s) => {
                    self.write_u64(STDERR::ERROR as u64).await?;
                    self.write_string(&msg).await?;
                    if s != 0 {
                        self.write_u64(s as u64).await?;
                    }
                }
                WorkFinish::Done => {
                    self.write_u64(STDERR::LAST as u64).await?;
                }
            }
            Ok(())
        }))
    }
}

pub enum WorkFinish {
    Done,
    Error(String, usize),
}

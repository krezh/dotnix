//! Connecting to the sound server and reading float samples from its record streams.

use std::ops::ControlFlow;

use libpulse_binding::{
    context::{Context, FlagSet, State},
    error::PAErr,
    mainloop::standard::{IterateResult, Mainloop},
    stream::{PeekResult, Stream},
};

pub fn connect(mainloop: &mut Mainloop, name: &str) -> Result<Context, Option<PAErr>> {
    let mut context = Context::new(mainloop, name).ok_or(None)?;
    context
        .connect(None, FlagSet::NOFAIL, None)
        .map_err(|_| None)?;
    loop {
        match mainloop.iterate(true) {
            IterateResult::Success(_) => {}
            IterateResult::Quit(_) | IterateResult::Err(_) => return Err(None),
        }
        match context.get_state() {
            State::Ready => return Ok(context),
            State::Failed | State::Terminated => return Err(Some(context.errno())),
            _ => {}
        }
    }
}

pub fn drain(
    stream: &mut Stream,
    mut sample: impl FnMut(f32) -> ControlFlow<()>,
) -> Result<ControlFlow<()>, PAErr> {
    loop {
        match stream.peek()? {
            PeekResult::Data(bytes) => {
                for chunk in bytes.chunks_exact(4) {
                    if sample(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                        .is_break()
                    {
                        return Ok(ControlFlow::Break(()));
                    }
                }
                let _ = stream.discard();
            }
            PeekResult::Hole(_) => {
                let _ = stream.discard();
            }
            PeekResult::Empty => return Ok(ControlFlow::Continue(())),
        }
    }
}

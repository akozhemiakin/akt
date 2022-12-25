use crate::ActorSendError;
use error_stack::{report, Context, ResultExt};

pub trait ActorResultIntoReport<R, C> {
    fn change_context<C2: Context>(self, context: C2) -> error_stack::Result<R, C2>;

    fn change_context_lazy<C2: Context, F: FnOnce() -> C2>(
        self,
        f: F,
    ) -> error_stack::Result<R, C2>;
}

impl<R, C> ActorResultIntoReport<R, C> for Result<error_stack::Result<R, C>, ActorSendError> {
    fn change_context<C2: Context>(self, context: C2) -> error_stack::Result<R, C2> {
        match self {
            Err(err) => Err(report!(err)).change_context_lazy(|| context),
            Ok(ok) => ok.change_context_lazy(|| context),
        }
    }

    fn change_context_lazy<C2: Context, F: FnOnce() -> C2>(
        self,
        f: F,
    ) -> error_stack::Result<R, C2> {
        match self {
            Err(err) => Err(report!(err)).change_context_lazy(f),
            Ok(ok) => ok.change_context_lazy(f),
        }
    }
}

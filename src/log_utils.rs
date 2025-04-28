use std::fmt::Display;

pub trait LogResult<T, E: Display> {
    fn log_ok(self, msg: &str) -> Option<T>;
}

impl<T, E: Display> LogResult<T, E> for Result<T, E> {
    fn log_ok(self, msg: &str) -> Option<T> {
        self.inspect_err(|e| log::error!("{msg}: {e}")).ok()
    }
}

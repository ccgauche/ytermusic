use flume::Sender;

use crate::term::{ManagerMessage, Screens};

pub fn handle_error_option<T, E>(
    updater: &Sender<ManagerMessage>,
    error_type: &'static str,
    a: Result<E, T>,
) -> Option<E>
where
    T: std::fmt::Display,
{
    match a {
        Ok(e) => Some(e),
        Err(a) => {
            updater
                .send(ManagerMessage::PassTo(
                    Screens::DeviceLost,
                    Box::new(ManagerMessage::Error(format!("{} {}", error_type, a))),
                ))
                .unwrap();
            None
        }
    }
}

pub fn handle_error<T>(updater: &Sender<ManagerMessage>, error_type: &'static str, a: Result<(), T>)
where
    T: std::fmt::Display,
{
    let _ = handle_error_option(updater, error_type, a);
}

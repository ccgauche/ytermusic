use flume::Sender;

use crate::term::{ManagerMessage, Screens};

/// Utils to handle errors
pub fn handle_error_option<T, E>(
    updater: &Sender<ManagerMessage>,
    error_type: &'static str,
    a: Result<E, T>,
) -> Option<E>
where
    T: std::fmt::Debug,
{
    match a {
        Ok(e) => Some(e),
        Err(a) => {
            updater
                .send(ManagerMessage::PassTo(
                    Screens::DeviceLost,
                    Box::new(ManagerMessage::Error(
                        format!("{error_type} {a:?}"),
                        Box::new(None),
                    )),
                ))
                .unwrap();
            None
        }
    }
}

/// Utils to handle errors
pub fn handle_error<T>(updater: &Sender<ManagerMessage>, error_type: &'static str, a: Result<(), T>)
where
    T: std::fmt::Debug,
{
    let _ = handle_error_option(updater, error_type, a);
}

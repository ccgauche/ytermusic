#[derive(Debug, Clone, PartialEq)]
pub enum MusicStatusAction {
    Skip(usize),
    Current,
    Before(usize),
    Downloading,
}

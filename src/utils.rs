use std::collections::VecDeque;

use ytpapi::Video;

use crate::systems::download::IN_DOWNLOAD;

// We don't know if current is bigger than 2 so we can't clamp or it can panic.
#[allow(clippy::manual_clamp)]
pub fn get_before(lines: usize, current: usize, size: usize) -> usize {
    // Remove the margin
    ((lines - 5).saturating_add(current).saturating_sub(size))
        .max(2)
        .min(current)
}

pub fn generate_music_repartition<'a>(
    lines: usize,
    queue: &'a VecDeque<Video>,
    previous: &'a [Video],
    current: &'a Option<Video>,
) -> (usize, usize) {
    let left =
        lines - current.as_ref().map(|_| 1).unwrap_or(0) - IN_DOWNLOAD.lock().unwrap().len() - 2;
    let before = previous.len().min(3);
    let left = left - before;
    let after = queue.len().min(left);
    let left = left - after;
    let before = before.max(left);
    (before, after)
}

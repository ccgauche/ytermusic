// We don't know if current is bigger than 2 so we can't clamp or it can panic.
#[allow(clippy::manual_clamp)]
pub fn get_before(lines: usize, current: usize, size: usize) -> usize {
    // Remove the margin
    ((lines - 5).saturating_add(current).saturating_sub(size))
        .max(2)
        .min(current)
}

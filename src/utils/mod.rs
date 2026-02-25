pub mod spinner;

pub fn evaluate_wrapping_index(current_index: isize, update: isize, len: isize) -> usize {
    ((current_index + update).rem_euclid(len)) as usize
}

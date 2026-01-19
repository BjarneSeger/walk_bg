//! Moderately useful functions

pub fn random_walk_step(x: u32, y: u32, width: u32, height: u32) -> (u32, u32) {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let mut hasher = RandomState::new().build_hasher();
    std::time::SystemTime::now().hash(&mut hasher);
    x.hash(&mut hasher);
    y.hash(&mut hasher);
    let random = hasher.finish();

    let direction = (random % 4) as u32;

    match direction {
        0 => (x, y.saturating_sub(1)),     // up
        1 => ((x + 1).min(width - 1), y),  // right
        2 => (x, (y + 1).min(height - 1)), // down
        3 => (x.saturating_sub(1), y),     // left
        _ => (x, y),
    }
}

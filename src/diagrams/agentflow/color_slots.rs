pub const KIND_SLOTS: [(&str, usize); 7] = [
    ("tool", 0),
    ("task", 1),
    ("decision", 2),
    ("input", 3),
    ("refdoc", 4),
    ("connector", 5),
    ("action", 6),
];

pub const KIND_COUNT: usize = KIND_SLOTS.len();

pub fn kind_slot(kind: &str) -> Option<usize> {
    KIND_SLOTS
        .iter()
        .find(|(name, _)| *name == kind)
        .map(|(_, slot)| *slot)
}

pub fn container_slot_count(palette_length: usize) -> usize {
    1usize.max(palette_length.saturating_sub(KIND_COUNT))
}

pub fn container_slot(n: usize, palette_length: usize) -> usize {
    let slot = KIND_COUNT + n % container_slot_count(palette_length);
    if palette_length > 0 {
        slot % palette_length
    } else {
        slot
    }
}

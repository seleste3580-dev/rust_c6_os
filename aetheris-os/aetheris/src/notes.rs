//! Equal-tempered note frequencies, names and chord tables.
pub const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const OCT4_X100: [u32; 12] = [
    26163, 27718, 29366, 31113, 32963, 34923, 36999, 39200, 41530, 44000, 46616, 49388,
];

/// Frequency in hundredths of Hz for a MIDI note (60 = C4).
pub fn freq_x100(midi: i32) -> u32 {
    let n = midi.clamp(12, 119);
    let oct = n / 12 - 1;
    let base = OCT4_X100[(n % 12) as usize];
    if oct >= 4 { base << (oct - 4) } else { base >> (4 - oct) }
}

/// Frequency in whole Hz (rounded).
pub fn freq(midi: i32) -> u32 {
    (freq_x100(midi) + 50) / 100
}

pub fn name(midi: i32) -> &'static str {
    NAMES[(midi.rem_euclid(12)) as usize]
}

pub fn octave(midi: i32) -> i32 {
    midi.div_euclid(12) - 1
}

pub fn is_black(midi: i32) -> bool {
    matches!(midi.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

pub struct ChordType {
    pub name: &'static str,
    pub iv: &'static [u8],
}

pub const CHORDS: [ChordType; 12] = [
    ChordType { name: "maj", iv: &[0, 4, 7] },
    ChordType { name: "min", iv: &[0, 3, 7] },
    ChordType { name: "7", iv: &[0, 4, 7, 10] },
    ChordType { name: "maj7", iv: &[0, 4, 7, 11] },
    ChordType { name: "m7", iv: &[0, 3, 7, 10] },
    ChordType { name: "dim", iv: &[0, 3, 6] },
    ChordType { name: "aug", iv: &[0, 4, 8] },
    ChordType { name: "sus2", iv: &[0, 2, 7] },
    ChordType { name: "sus4", iv: &[0, 5, 7] },
    ChordType { name: "6", iv: &[0, 4, 7, 9] },
    ChordType { name: "m6", iv: &[0, 3, 7, 9] },
    ChordType { name: "add9", iv: &[0, 4, 7, 14] },
];

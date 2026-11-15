//! Sys context: shared system state passed to every app.
//!
//! Holds: settings, Wi-Fi state, IR snapshot, hardware RNG, sound/LED/save effect queue.
//! Apps interact with hardware exclusively through `Sys` — they never touch GPIOs directly.
//!
//! Stub: full implementation lives in the git repo.
//! Run `cd ~/rust_c6_os && git checkout .` to restore all original source files.

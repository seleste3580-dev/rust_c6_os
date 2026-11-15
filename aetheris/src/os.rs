//! OS Shell: boot sequence, login, home screen, app grid, lock, sleep, and power management.
//!
//! The shell manages the two framebuffers (OLED A + OLED B), dispatches key events
//! to the active app, and renders wallpapers, status bar (clock, Wi-Fi icon, volume),
//! and the BIOS self-test / splash / login / home / menu / lock / sleep states.
//!
//! Stub: full implementation lives in the git repo.
//! Run `cd ~/rust_c6_os && git checkout .` to restore all original source files.

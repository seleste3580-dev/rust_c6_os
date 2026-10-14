# Aetheris PC simulator

Runs the *hardware-independent* half of the OS (shell, apps, UI, settings codec, NEC decoder...) on your PC.

```sh
cd aetheris-sim
cargo test                                   # unit tests (multi-tap, notes, settings, NEC, key scanner...)
python3 tools/make_demo_script.py         # writes demo.txt (boot -> login -> menu)
cargo run -- demo.txt frames.bin             # simulate; dump both OLED framebuffers at each `snap`
python3 tools/render_png.py frames.bin sheet.png   # needs `pip install pillow`
```
Script format: one event per line: `<ms> <key> [p|l|r]` (press/long/release) or `<ms> snap`.

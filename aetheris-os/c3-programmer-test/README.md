# ESP32-C3 programmer test

The release firmware in `target/riscv32imc-unknown-none-elf/release/c3-programmer-test`
toggles GPIO7 every 500 ms. Connect an LED anode through a 330-ohm resistor to GPIO7
and the cathode to GND to observe a successful boot.

The app-only binary is generated at `slave-c3-test.bin` in this directory with:

```sh
cargo build --release
espflash save-image --chip esp32c3 target/riscv32imc-unknown-none-elf/release/c3-programmer-test slave-c3-test.bin
```

Do not add `--merge`. The programmer flashes app-only images at `0x10000` and leaves
the target bootloader and partition table unchanged. This is a test image for a
standard ESP32-C3 app partition beginning at `0x10000`.

## Connect the boards

Connect the Aetheris ESP32-C6 host to the ESP32-C3 target:

| Aetheris ESP32-C6 | ESP32-C3 target | Purpose |
|---|---|---|
| GPIO8 (UART TX) | RX (board label) | Firmware data to C3 |
| GPIO9 (UART RX) | TX (board label) | C3 replies to programmer |
| GPIO5 | GPIO9 / BOOT (board label) | Select ROM download mode |
| GPIO15 | EN (board label) | Reset target |
| GND | GND | Common ground |

The board's RX/TX labels are its UART0 pins; use those labels instead of looking
for GPIO20/21. Power the C3 from a suitable regulated 5 V supply connected to its
5V pin, or from the C6 5V pin only when the C6 itself is powered from USB and that
USB supply has enough current for both boards. Connect the grounds. Do not connect
the 3V3 pins together, and do not connect another 5 V source while using the C6's
5V pin. Both boards use 3.3 V UART logic. The programmer uses C6 GPIO15 to reset
the target; GPIO15 is a C6 strapping pin, so leave the C3 EN pull-up in place.
Keep the C3 UART high-impedance while the C6 is resetting because C6 GPIO8/9 are
strapping pins.

Join Aetheris to Wi-Fi, open **OTA / Programmer**, press `2` or `8` to select
slave mode, use `4`/`6` to select **ESP32-C3**, then press `#` to enable the server.
From a computer on the same network, upload:

```sh
curl --data-binary @slave-c3-test.bin http://<aetheris-ip>/program/c3
```

On success, the C3 resets into the test app and GPIO7 toggles. A power loss while
the target app is being written may require reflashing the C3 over USB; the host
firmware and settings are not written by the slave programmer.

# Super Rustycom

An ongoing SNES-emulator project written in Rust. This won't be very accurate and it probably won't be very fast, but it would be nice to get some demos running.

Features so far:
- 66c816
- RAM, ROM (vanilla LoROM)
- Most of A-bus registers, though untested
- "Dummy" APU
 - SMP executes, registers are updated
 - No DSP
- Debug draw
 - CPU disassembly for recent history
 - CPU state
 - RAM
 - SMP state

Big shoutouts to [ferris](https://github.com/yupferris) for [lighting the spark](https://www.youtube.com/playlist?list=PL-sXmdrqqYYcL2Pvx9j7dwmdLqY7Mx8VY).

The stuff I've used as reference so far:\
[65c816 opcodes](http://6502.org/tutorials/65c816opcodes.html)\
[nocash specs](http://problemkaputt.de/fullsnes.htm)\
[SFC Development Wiki](https://wiki.superfamicom.org/)\
[Geiger's Snes9x Debugger](https://www.romhacking.net/utilities/241/)

## Building

Included debug build task requires nightly for macro debug toys.

## Running

Run with `super_rustycom(.exe) --rom {rom_path}`. Subsequent runs don't require the argument to load the same rom as the previous one is loaded from config.json as a fallback.

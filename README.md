# Super Rustycom

This is my ongoing SNES-emulator project written in Rust. So far I've implemented the 65c816 instruction set with no timing as well as the main WRAM, some control registers and running a basic LoROM with no additional chips. There is also a debugger/disassembler that currently controls the cpu. I've made unit tests for select functionality while the first "real" goal has been to get [nu by elix](http://www.pouet.net/prod.php?which=62927) running. Big shoutouts to [ferris](https://github.com/yupferris) for [lighting the spark](https://www.youtube.com/playlist?list=PL-sXmdrqqYYcL2Pvx9j7dwmdLqY7Mx8VY).

The stuff I've used as reference so far:\
[65c816 opcodes](http://6502.org/tutorials/65c816opcodes.html)\
[nocash specs](http://problemkaputt.de/fullsnes.htm)\
[SFC Development Wiki](https://wiki.superfamicom.org/)\
[Geiger's Snes9x Debugger](https://www.romhacking.net/utilities/241/)

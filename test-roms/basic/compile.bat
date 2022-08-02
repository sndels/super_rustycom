ca65 --cpu 65816 -o main.o main.asm
ld65 -C memmap.cfg main.o -o basic.sfc

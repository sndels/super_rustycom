; Base from georgjz's tutorial

; Setup SNES env
.p816
.i16
.a8

.segment "CODE"
.proc   ResetHandler            ; program entry point
        sei                     ; disable interrupts
        clc                     ; clear the carry flag...
        xce                     ; ...and switch to native mode

        lda #$81                ; enable...
        sta $4200               ; ...non-maskable interrupt

        clc
        lda #$DB
        adc #$23

        jmp GameLoop            ; initialisation done, jump to game loop
.endproc

.proc   GameLoop                ; The main game loop
        wai                     ; wait for NMI interrupt
        jmp GameLoop            ; jump to beginning of main game loop
.endproc

.proc   NMIHandler              ; NMIHandler, called every frame/V-blank
        lda $4210               ; read NMI status
        rti                     ; interrupt done, return to main game loop
.endproc

.segment "VECTOR"
; native mode   COP,        BRK,        ABT,
.addr           $0000,      $0000,      $0000
;               NMI,        RST,        IRQ
.addr           NMIHandler, $0000,      $0000

.word           $0000, $0000    ; four unused bytes

; emulation m.  COP,        BRK,        ABT,
.addr           $0000,      $0000,      $0000
;               NMI,        RST,        IRQ
.addr           $0000 ,     ResetHandler, $0000

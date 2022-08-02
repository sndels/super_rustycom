; Base from georgjz's tutorial

; Registers
INIDISP     = $2100     ; inital settings for screen
OBJSEL      = $2101     ; object size $ object data area designation
OAMADDL     = $2102     ; address for accessing OAM
OAMADDH     = $2103
OAMDATA     = $2104     ; data for OAM write
VMAINC      = $2115     ; VRAM address increment value designation
VMADDL      = $2116     ; address for VRAM read and write
VMADDH      = $2117
VMDATAL     = $2118     ; data for VRAM write
VMDATAH     = $2119     ; data for VRAM write
CGADD       = $2121     ; address for CGRAM read and write
CGDATA      = $2122     ; data for CGRAM write
TM          = $212c     ; main screen designation
NMITIMEN    = $4200     ; enable flaog for v-blank
RDNMI       = $4210     ; read the NMI flag status

; Setup SNES env
.p816

;----- Includes ----------------------------------------------------------------
.segment "SPRITEDATA"
SpriteData: .incbin "sprite.vram"
PaletteData:  .incbin "sprite.cgram"

.segment "CODE"
;-------------------------------------------------------------------------------
;   This is the entry point
;-------------------------------------------------------------------------------
.proc   ResetHandler            ; program entry point
        sei                     ; disable interrupts
        clc                     ; clear the carry flag...
        xce                     ; ...and switch to native mode
        lda #$8f                ; force v-blanking
        sta INIDISP
        stz NMITIMEN            ; disable NMI


        ; transfer vram data
        stz VMADDL              ; set VRAM addr to $0000
        stz VMADDH
        lda #$80
        sta VMAINC              ; increment VRAM addr by 1 when writing to VMDATAH
        ldx #$00                ; loop counter and offset
VRAMLoop: ; copy sprite, rows from two planes at a time
        lda SpriteData, X       ; write bitplane 0/2 byte to VRAM
        sta VMDATAL
        inx
        lda SpriteData, X       ; write bitplane 1/3 byte to VRAM
        sta VMDATAH
        inx
        cpx #$20                ; check if we have written the full sprite
                                ; 4 bitplanes with 8 rows each, byte per row
        bcc VRAMLoop

        ; transfer cgram data
        lda #$80                ; set CGRAM addr to $80
        sta CGADD
        ldx #$00                ; loop counter and offset
CGRAMLoop: ; copy palette word by word
        lda PaletteData, X
        sta CGDATA
        inx
        lda PaletteData, X
        sta CGDATA
        inx
        cpx #$20                ; check if we have written the full palette (16 words)
        bcc CGRAMLoop

        ; set up OAM data
        stz OAMADDL             ; set OAM addr to $0000
        stz OAMADDH
        lda # (256 / 2 - 8)     ; x pos
        sta OAMDATA
        lda # (224 / 2 - 8)     ; y pos
        sta OAMDATA
        lda #$00                ; name
        sta OAMDATA
        lda #$00                ; no flip, prio 0, palette 0
        sta OAMDATA

        ; make objects visible
        lda #$10
        sta TM
        ; release forced blanking, set screen to full brightness
        lda #$0f
        sta INIDISP
        ; enable NMI, turn on joypad polling
        lda #$81
        sta NMITIMEN

        jmp GameLoop





        jmp GameLoop            ; initialisation done
.endproc
;-------------------------------------------------------------------------------

;-------------------------------------------------------------------------------
;   After the ResetHandler will jump to here
;-------------------------------------------------------------------------------
; .smart ; keep track of registers widths
.proc   GameLoop
        wai                     ; wait for NMI / V-Blank

        ; here we would place all of the game logic
        ; and loop forever

        jmp GameLoop
.endproc
;-------------------------------------------------------------------------------

;-------------------------------------------------------------------------------
;   Will be called during V-Blank
;-------------------------------------------------------------------------------
.proc   NMIHandler
        lda RDNMI               ; read NMI status, acknowledge NMI

        ; this is where we would do graphics update

        rti
.endproc
;-------------------------------------------------------------------------------

;-------------------------------------------------------------------------------
;   Is not used in this program
;-------------------------------------------------------------------------------
.proc   IRQHandler
        ; code
        rti
.endproc
;-------------------------------------------------------------------------------

;-------------------------------------------------------------------------------
;   Interrupt and Reset vectors for the 65816 CPU
;-------------------------------------------------------------------------------
.segment "VECTOR"
; native mode   COP,        BRK,        ABT,
.addr           $0000,      $0000,      $0000
;               NMI,        RST,        IRQ
.addr           NMIHandler, $0000,      $0000

.word           $0000, $0000    ; four unused bytes

; emulation m.  COP,        BRK,        ABT,
.addr           $0000,      $0000,      $0000
;               NMI,        RST,        IRQ
.addr           $0000,      ResetHandler, $0000

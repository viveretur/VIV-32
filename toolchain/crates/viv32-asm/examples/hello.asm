; Demo program for testing assembler.
.text:
.org 0x0
    jmp _start                 ; Skip the interrupt vector table

.const SERIAL_TX_EN   0x0003
.const SERIAL_ADDRESS 0xFFFF0200

.nomangle _end
.org 0x7C                      ; Any interrupt will halt.
_end:
    halt

_start:
    lli     $1, SERIAL_TX_EN   ; Serial control mask to enable TX
    li      $2, SERIAL_ADDRESS ; Serial MMIO base address
    sw      $1, [$2, 4]        ; Apply serial control mask

loop:
    lbu     $1, [$3, message]  ; Read offset byte of message
    b.eq    $1, $0, _end       ; Exit if NUL
    sb      $1, [$2, 0]        ; Send byte to serial data buffer
    addi    $3, $3, 1          ; Increment index
    jmp     loop

.rodata:
.org 0x0100
message_ptr:  .uword message
message:    .asciz  "Hello World!\n"
tx_map:   .uword  SERIAL_TX_EN

.bss:
data:     .space 4, 32

; Demo program for testing assembler.
.text:
.org 0x0
    jmp _start                         ; Skip the interrupt vector table

; Shared definitions from serial_tx.asm
.nomangle serial_enable_tx
.nomangle serial_disable_tx
.nomangle serial_buffer_tx_byte
.nomangle serial_tx

; In case we ever want to load it
.nomangle _start

.org 0x7C                               ; Any interrupt will halt.
_end:
    halt

_start:
    call serial_enable_tx
    
    la      $2, message_ptr             ; Loads pointer to  message's first byte
loop:
    lbu     $1, [$2, 0]                 ; Load the byte into memory
    b.eq    $1, $0, exit_loop           ; Exit loop at end of byte
    call    serial_buffer_tx_byte       ; Send the byte to the serial buffer
    inc     $2                          ; Move to next byte of message
    jmp     loop

exit_loop:
    call serial_tx
    call serial_disable_tx
    jmp _end

.rodata:
.org 0x0100
message_ptr:  .uword message
message:      .asciz "Hello World!\n"

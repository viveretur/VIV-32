; Demo program for testing assembler.

; Shared definitions from serial_tx.asm and serial spec.
.nomangle serial_set_control
.nomangle serial_has_byte
.nomangle serial_read
.nomangle serial_send
.const SERIAL_DUPLEX    0x17            ; Enabled for duplex transmission.
.const SERIAL_TX_READY  0x2             ; Mask for testing

; Data stored in interrupt routines
.nomangle serial_buffer
.nomangle serial_buffer_len

.nomangle _main
_main:
    li      %sp, 8192                   ; Initialise the stack pointer
    lli     $1, SERIAL_DUPLEX
    call    serial_set_control
    ei

loop:
    call    serial_has_byte
    b.eq    $2, $0, loop

    call    serial_read
    b.eq    $2, $0, _end
    mov     $1, $2

send_loop:
    call    serial_send
    b.ne    $2, $0, send_loop           ; Serial could not send, retry
    jmp     loop    
    
_end:
    halt

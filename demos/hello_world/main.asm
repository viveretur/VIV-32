; Demo program for testing assembler.
.text:
.org 0x0
    jmp     _start                      ; Skip the interrupt vector table

; Shared definitions from serial_tx.asm and serial spec.
.nomangle serial_set_control
.nomangle serial_status
.nomangle serial_send
.const SERIAL_SEND      0x3             ; Enabled and send enabled.
.const SERIAL_TX_READY  0x2             ; Mask for testing

.org 0x7C                               ; Any interrupt will halt.
_end:
    halt

_start:
    li      %sp, 1024                   ; Initialise the stack pointer
    lli     $1, SERIAL_SEND             ; Set serial send mode
    call    serial_set_control
    
    la      $8, message_ptr             ; Loads pointer to  message's first byte
    lw      $8, [$8, 0]                 ; Loads the address of the message
message_loop:
    lbu     $1, [$8, 0]                 ; Load the byte into memory
    b.eq    $1, $0, exit_loop           ; Exit loop at end of message

send_loop:
    call    serial_status               ; Loads the status into $2
    andi    $2, $2, SERIAL_TX_READY
    b.eq    $2, $0, send_loop           ; Busy loop if not ready

    call    serial_send                 ; Send $1 to the serial device
    inc     $8                          ; Move to next byte of message
    jmp     message_loop    

exit_loop:
    mov     $1, $0
    call    serial_set_control          ; Disable the serial device
    jmp _end

.rodata:
message_ptr:  .uword message
message:      .asciz "Hello World!\n"

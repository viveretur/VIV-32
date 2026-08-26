; Demo echo program for testing assembler
; Duplicates input upon buffer flush

; Shared definitions from serial_tx.asm and serial spec
.nomangle serial_set_control
.nomangle serial_has_byte
.nomangle serial_read
.nomangle serial_send
.const SERIAL_DUPLEX    0x17            ; Enables duplex transmission and ISR

.nomangle _main                         ; Called by the reset vector at 0x00
_main:
    li      %sp, 8192                   ; Initialise the stack pointer
    lli     $1, SERIAL_DUPLEX           ; Sets argument for serial control
    call    serial_set_control
    ei                                  ; Enable interrupts after enabling ISR

loop:
    call    serial_has_byte
    b.eq    $2, $0, loop                ; Loop until a byte is recieved

    call    serial_read
    b.eq    $2, $0, _end                ; Check for EOF
    mov     $1, $2                      ; Prepare to send byte

send_loop:
    call    serial_send
    b.ne    $2, $0, send_loop           ; Serial could not send, retry
    jmp     loop
    
_end:
    halt

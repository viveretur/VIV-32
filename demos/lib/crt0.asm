.text:
.org 0x00
    jmp     _start

.org 0x60
    jmp     service_interrupt

.org 0x7C
    halt

.nomangle serial_isr

; The serial address will be in edata
.const SERIAL_ADDRESS   0xFFFF0200
service_interrupt:
    push    $1, $2, %lr
    mrs     $1, %edata
    li      $2, SERIAL_ADDRESS
    b.ne    $1, $2, service_interrupt_end

    call    serial_isr

service_interrupt_end:
    pop     %lr, $2, $1
    iret

; Entry point to the program. This wraps the call to main, then uses the
; serial library to print out the return value in $2.
.nomangle main
.nomangle serial_set_control
.nomangle serial_send
.const SERIAL_SEND 0x3

_start:
    li      %sp, 8192                   ; Set the stack pointer
    call    main
    
    push    $0                          ; Set a sentinel for reverse display
    lli     $6, 10                      ; Push newline
    push    $6
    lli     $3, 10                      ; Display results in base 10
    lli     $4, 0x30                    ; Offset for numeric display
    lli     $1, SERIAL_SEND             ; enable the serial transmission
    call    serial_set_control

_calc:
    divu    $2, $1, $2, $3              ; mod in $1, quotient remains in $2
    add     $1, $1, $4                  ; convert to ASCII
    push    $1                          ; push onto stack to reverse later
    b.gtu   $2, $0, _calc               ; continue as long as values are left

    lli     $1, SERIAL_SEND             ; enable the serial transmission
    call    serial_set_control

_disp:
    pop     $1                          ; pull largest ordinal off stack
    b.eq    $1, $0, _end                ; check for sentinel
_send:
    call    serial_send
    b.ne    $2, $0, _send               ; Could not send (likely busy), retry
    jmp     _disp                       ; get next value

_end:
    halt

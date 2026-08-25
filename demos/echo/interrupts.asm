.text:
.org 0x00
.nomangle _main
    jmp     _main

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

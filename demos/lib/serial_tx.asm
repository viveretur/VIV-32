; File #2 for linker testing.

; These values must align with device_config and the serial specification.
.const SERIAL_DATA      0xFFFF0200
.const SERIAL_CONTROL   0xFFFF0204
.const SERIAL_STATUS    0xFFFF0208
.const STATUS_RX_READY  0x1
.const STATUS_TX_READY  0x2

.nomangle serial_set_control
.nomangle serial_status
.nomangle serial_read
.nomangle serial_send
.nomangle serial_isr
.nomangle serial_has_byte

; Set value, from $1
serial_set_control:
    di
    push    $2
    
    li      $2, SERIAL_CONTROL
    sw      $1, [$2, 0]

    pop     $2
    ei
    ret

; Retrieve the current status, into $2
serial_status:
    li      $2, SERIAL_STATUS
    lw      $2, [$2, 0]
    ret

; Returns 0 if no data, other value if not, into $2
serial_has_byte:
    push    $1, $3
    la      $1, serial_buffer_head
    lb      $1, [$1, 0]
    la      $2, serial_buffer_tail
    lb      $2, [$2, 0]
    sub     $2, $2, $1
    pop     $3, $1
    ret
 
; Retrieve the oldest byte in the buffer, into $2. If there is none, reads 0.
serial_read:
    push    %lr
    call    serial_has_byte
    pop     %lr
    b.eq    $2, $0, serial_read_end

    push    $1, $3, $4
    la      $1, serial_buffer
    la      $3, serial_buffer_tail
    lb      $4, [$3, 0]
    add     $1, $1, $4
    lb      $2, [$1, 0]
    inc     $4
    sb      $4, [$3, 0]
    pop     $4, $3, $1

serial_read_end:
    ret

; Send the current byte. Sets $2 = 0 if successful, $2 = 1 if not.
serial_send:
    li      $2, SERIAL_STATUS
    lw      $2, [$2, 0]
    andi    $2, $2, STATUS_TX_READY
    b.eq    $2, $0, ser_not_ready       ; SERIAL_TX is high if ready

    li      $2, SERIAL_DATA
    sb      $1, [$2, 0]
    clr     $2
    ret

ser_not_ready:
    li      $2, 1
    ret   

serial_isr:
    push    $1, $2, $3, $4, %lr

    call    serial_status
    andi    $2, $2, STATUS_RX_READY
    b.eq    $2, $0, serial_isr_end
    
    li      $1, SERIAL_DATA
    lb      $1, [$1, 0]
    la      $2, serial_buffer
    la      $3, serial_buffer_head
    lb      $4, [$3, 0]
    add     $2, $2, $4
    sb      $1, [$2, 0]
    inc     $4
    sb      $4, [$3, 0]

serial_isr_end:
    pop     %lr, $4, $3, $2, $1
    ret

.data
serial_buffer_head: .ubyte 0
serial_buffer_tail: .ubyte 0
serial_buffer: .space 1, 256

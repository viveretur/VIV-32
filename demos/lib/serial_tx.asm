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
    push    $2                          ; Protect used register
    li      $2, SERIAL_CONTROL          ; Load the control's MMIO address
    sw      $1, [$2, 0]                 ; Send the configuration via MMIO
    pop     $2                          ; Restore used register
    ret

; Retrieve the current status, into $2
serial_status:
    li      $2, SERIAL_STATUS           ; Load the status's MMIO address
    lw      $2, [$2, 0]                 ; Read it into the return register
    ret

; Returns 0 if no data, other value if not, into $2
serial_has_byte:
    push    $1, $3                      ; Protect used registers
    la      $1, serial_buffer_head      ; Load the ring buffer head pointer
    lb      $1, [$1, 0]                 ; Get its offset
    la      $2, serial_buffer_tail      ; Load the ring buffer tail pointer
    lb      $2, [$2, 0]                 ; Get its offset
    sub     $2, $2, $1                  ; Put the difference in return register
    pop     $3, $1                      ; Restore protected registers
    ret
 
; Retrieve the oldest byte in the buffer, into $2. If there is none, reads 0.
serial_read:
    push    %lr                         ; Protect own call return register
    call    serial_has_byte             ; because this call will wipe it
    pop     %lr                         ; Restore after call
    b.eq    $2, $0, serial_read_end     ; No data, so go to return

    push    $1, $3, $4                  ; Protect used registers
    la      $1, serial_buffer           ; $1 holds the ring buffer base address
    la      $3, serial_buffer_tail      ; $3 holds pointer to tail offset
    lb      $4, [$3, 0]                 ; Read tail offset value into $4
    add     $1, $1, $4                  ; Add ring base + offset for tail locn
    lb      $2, [$1, 0]                 ; Read tail byte into return reg
    inc     $4                          ; Increment tail offset value
    sb      $4, [$3, 0]                 ; Save tail offset value back, wrapped
    pop     $4, $3, $1                  ; Restore used registers

serial_read_end:
    ret

; Send the byte in $1. Sets $2 = 0 if successful, $2 = 1 if not.
serial_send:
    li      $2, SERIAL_STATUS           ; Load MMIO address of status
    lw      $2, [$2, 0]                 ; Read the status
    andi    $2, $2, STATUS_TX_READY     ; Check if its ready, TX_READY is 1
    b.eq    $2, $0, ser_not_ready       ; Skip send if serial tx not ready

    li      $2, SERIAL_DATA             ; Load MMIO address of serial data byte
    sb      $1, [$2, 0]                 ; Write the byte, triggering transmit
    clr     $2                          ; Clear for success value return
    ret

ser_not_ready:
    li      $2, 1                       ; Load error value into return
    ret

; Interrupt handler. Loads received byte into ring buffer.
serial_isr:
    push    $1, $2, $3, $4, %lr         ; Protect used registers

    call    serial_status               ; Read status register, check for recv
    andi    $2, $2, STATUS_RX_READY     ; RX_READY is 1 if there's data to read
    b.eq    $2, $0, serial_isr_end      ; If no data, return is 0, skip to end
    
    li      $1, SERIAL_DATA             ; Load MMIO address of serial data byte
    lb      $1, [$1, 0]                 ; Read the byte
    la      $2, serial_buffer           ; Load address of ring buffer into $2
    la      $3, serial_buffer_head      ; Load address of buffer head into $3
    lb      $4, [$3, 0]                 ; Read head offset into $4
    add     $2, $2, $4                  ; Calculate base+offset into $2
    sb      $1, [$2, 0]                 ; Store the read byte to base+offset $2
    inc     $4                          ; Increment the offset value by 1
    sb      $4, [$3, 0]                 ; Write the offset value back, wrapping

serial_isr_end:
    pop     %lr, $4, $3, $2, $1         ; Restore used registers
    ret

.data
serial_buffer_head: .ubyte 0
serial_buffer_tail: .ubyte 0
serial_buffer: .space 1, 256

; File #2 for linker testing.

.const SERIAL_TX_EN   0x0003
.const SERIAL_ADDRESS 0xFFFF0200

.nomangle serial_enable_tx
.nomangle serial_disable_tx
.nomangle serial_buffer_tx_byte
.nomangle serial_tx

; Enable the serial output
serial_enable_tx:
    push    $1, $2
    lli     $1, SERIAL_TX_EN            ; Serial control mask to enable TX
    li      $2, SERIAL_ADDRESS          ; Serial MMIO base address
    sw      $1, [$2, 4]                 ; Apply serial control mask
    sb      $0, [$0, serial_data]       ; Initialize serial buffer length to 0
    pop     $2, $1
    ret

; Disable the serial output
serial_disable_tx:
    push    $1
    li      $1, SERIAL_ADDRESS
    sw      $0, [$1, 4]                 ; Wipes serial control mask
    pop     $1
    ret

; Byte to send should be in $1
serial_buffer_tx_byte:
    push    $2, $3
    la      $2, serial_data             ; Load address of the bss serial buffer
    lb      $3, [$2, 0]                 ; Read the count out of the buffer
    add     $3, $3, $2                  ; Find the last storage location
    inc     $3                          ; Calculate the new storage location
    sb      $1, [$3, 0]                 ; Save the byte into the bss buffer
    sb      $3, [$2, 0]                 ; Save the increased length back
    pop     $3, $2
    ret

; Iterates through the buffer, sending all bytes
serial_tx:
    push    $1, $2, $3, $4
    la      $2, serial_data             ; Load address of the bss serial buffer
    lb      $3, [$2, 0]                 ; Read the count out of the buffer
    add     $3, $3, $2                  ; Calculate the last storage location
    li      $4, SERIAL_ADDRESS          ; Load serial transmit buffer address
serial_tx_loop:
    inc     $2
    lb      $1, [$2, 0]                 ; Read the current byte
    sb      $1, [$4, 0]                 ; Send the byte to the serial tx MMIO
    b.ne    $2, $3, serial_tx_loop      ; Check if last byte, loop if not
    la      $2, serial_data
    sb      $0, [$2, 0]                 ; Clear buffer count
    pop     $4, $3, $2, $1
    ret
    
.bss:
serial_data:     .space 1, 256


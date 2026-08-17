ASM := cargo run -q -p viv32-asm --
LD  := cargo run -q -p viv32-ld --

ASM_SRCS := hello.asm serial_tx.asm
OBJS := $(ASM_SRCS:.asm=.vo)

BIN := hello.bin

.PHONY: all clean run

all: $(BIN)

$(BIN): $(OBJS)
	$(LD) $(OBJS) -o $@

%.vo: %.asm
	$(ASM) $< -o $@

clean:
	rm -f $(OBJS) $(BIN)

EXE        = kingly

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else
	NAME := $(EXE)
endif

rule:
	cargo build --release
	cp ../target/release/kingly $(NAME)

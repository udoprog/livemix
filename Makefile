.PHONY: run clean

target:
	mkdir -p target

target/%: target examples/%.c
	gcc -g -Wall -lm examples/$*.c -o $@ $(shell pkg-config --cflags --libs libpipewire-0.3)

clean:
	rm -f target/test
	rm -f target/node

run: target/node
	PIPEWIRE_DEBUG=D target/node

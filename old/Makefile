.PHONY: run clean

target:
	mkdir -p target

examples/stream.o: examples/stream.c
	gcc -g -Wall -c examples/stream.c -o $@ $(shell pkg-config --cflags --libs libpipewire-0.3)

target/%: target examples/%.c
	gcc -g -Wall -lm examples/$*.c -o $@ $(shell pkg-config --cflags --libs libpipewire-0.3)

clean:
	rm -f target/test
	rm -f target/node

run: target/export
	target/export

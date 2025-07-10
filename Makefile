.PHONY: run

examples/%: examples/%.c
	gcc -Wall -lm $< -o $@ $(shell pkg-config --cflags --libs libpipewire-0.3)

run: test
	./test
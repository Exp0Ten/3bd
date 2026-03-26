all: example

example: main.c other.c
	gcc main.c other.c -g -o example

opti: main.c other.c
	gcc main.c other.c -g -Os -o example

clean:
	rm -f example

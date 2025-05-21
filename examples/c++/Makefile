all: example

example: main.cpp other.cpp
	g++ main.cpp other.cpp -g -o example

opti: main.cpp other.cpp
	g++ main.cpp other.cpp -g -Os -o example

clean:
	rm -f example

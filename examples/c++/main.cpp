#include <iostream>
#include "other.h"

int factorial(int n) {
	if (n == 0) {return 1;}
	int next = factorial(n-1);
	return next*n;
}

int main() {

	int a = 10;

	int b = factorial(a);

	b += a;

    std::cout<<"Hello World\n";
    test_input();
    return 0;
}

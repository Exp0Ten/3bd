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

    for (int i = 0; i < 100000000; i++) {
        i += 1;
    }



    std::cout<<"Hello World\n";
    test_func();
    return 0;
}

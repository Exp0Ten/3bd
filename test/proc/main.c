enum What {
	How,
	Why,
	When,
	
};

typedef struct {
	unsigned int a;
	enum What n;
} Hello;


int main() {
	int x = 10;
	int y = 20;
	int a = x +y;

	enum What n = How;

	Hello m = (Hello) {
		.a = a,
		.n = n
	};

	for (int i = 0; i < 1; i) {
		
	}
}

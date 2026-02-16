enum Greeting {
    Hi,
    Hey,
    Hello,
    GoodMorning
}


struct Hello {
    name: Greeting,
    length: u64,
    text: String
}

impl Hello {
    fn default() -> Self {
        Hello {
            name: Greeting::Hi,
            length: 0,
            text: String::new()
        }
    }
}

static h_string: &str = "Hello World!\n";


mod display;

fn main() {

    let bomba = display::ahoj {
        x: 10,
        y: 20
    };

    fn wow(a: u64) {
        println!("wow");
    }

    let mut hello = Hello::default();

    let text = String::from("Good morning everyone");

    let a = &text;
    let c = a;

    let length = text.len();

    hello.text = text;
    hello.length = display::a(length as u8, 0) as u64;

    wow(10);
}

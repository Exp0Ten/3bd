
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
        let a = 10;
        let b = 30;
        let c = a + b;
        println!("{c}");
        Hello {
            name: Greeting::Hi,
            length: 0,
            text: String::new()
        }
    }

    fn change_name(&mut self, name: Greeting) {
        let wow = name;
        self.name = wow;
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

    let o = Hello::default();
    let a = &text;
    let c = a;

    let length = text.len();

    hello.text = text;
    hello.length = display::a(length as u8, 0) as u64;

    hello.change_name(Greeting::GoodMorning);

    wow(10);
}

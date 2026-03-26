mod display;

type Uni = (u8, u64);
static hello_string: &str = "Hello World!\n";

enum Greeting {
    Hi,
    Hey,
    Hello,
    GoodMorning
}

struct Message {
    greeting: Greeting,
    text: String,
    length: u64
}

impl Message {
    fn default() -> Self {
        let a = 10;
        let b = 30;
        let c = a + b;
        println!("{c}");
        Self {
            greeting: Greeting::Hi,
            text: String::new(),
            length: 0
        }
    }

    fn change_greet(&mut self, greet: Greeting) {
        self.greeting = greet;
    }
}


fn main() {
    let coords = display::Pixel {
        x: 10,
        y: 20
    };

    let my_13_array = [13;13];

    let mut message = Message::default();

    let text = String::from(hello_string);

    let a = &text;
    let c = a;
    let x: Uni = (10, 65);

    let length = text.len();

    message.text = text;
    message.length = display::add_byte(length as u8, 20) as u64;

    message.change_greet(Greeting::GoodMorning);

    fn wow() {
        println!("wow");
    }

    wow();
}

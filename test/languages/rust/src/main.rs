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

    let mut hello = Hello::default();

    let text = String::from("Good morning everyone");

    let length = text.len();

    hello.text = text;
    hello.length = display::a(length as u8, 0) as u64;


}

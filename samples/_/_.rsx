
struct Hello {
    name: String,
}

impl Hello {
    fn hello(&self) {
        println!("Hello {}", self.name);
    }
}

fn main() {
    // original rust style
    // let a = Hello { name: "RSX".to_string() };

    // rsx: allow partial initializing
    let a: Hello;
    a.name = "RSX".to_string();

    a.hello();
}

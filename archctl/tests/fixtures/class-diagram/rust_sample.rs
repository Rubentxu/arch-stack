//! Sample Rust crate for class-diagram fixture.

pub struct UserService {
    pub name: String,
    email: String,
}

impl UserService {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            email: format!("{}@example.com", name),
        }
    }

    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

pub trait Greetable {
    fn greet(&self) -> String;
}

impl Greetable for UserService {
    fn greet(&self) -> String {
        self.greet()
    }
}

pub struct AdminService {
    pub level: u8,
}

impl AdminService {
    pub fn new() -> Self {
        Self { level: 0 }
    }
}

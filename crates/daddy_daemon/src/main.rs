use std::{fs::File, io::Write};

use daddy::{store::Store, values::Value};

fn s (s: &str) -> Value {
    Value::String(s.to_string())
}
fn i (i: i64) -> Value {
    Value::Int(i)
}
fn b (b: bool) -> Value {
    Value::Bool(b)
}
fn bin (b: &[u8]) -> Value {
    Value::Binary(b.to_vec())
}
fn c (c: char) -> Value {
    Value::Ch(c)
}

fn main() {
    let mut store = Store::new();
    
    store.insert(s("Date"), i(4));
    store.insert(s("Month"), i(1));
    store.insert(s("Year"), i(2006));
    store.insert(s("Is Pretty Sick"), b(true));
    store.insert(i(69), s("Funny Sex Number"));
    store.insert(b(false), bin(&(0xDEADBEEF_u32.to_le_bytes())));

    let serialised = store.ser().unwrap();
    
    let mut file = File::create("db.ddb").unwrap();
    file.write_all(&serialised).unwrap();
}

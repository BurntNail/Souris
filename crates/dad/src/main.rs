#![allow(dead_code)]

use core::panic;
use std::{
    fs::File,
    io::{Read, Write},
};

use daddy::{store::Store, values::Value};
use daddy::niches::integer::Integer;

fn s(s: &str) -> Value {
    Value::String(s.to_string())
}
fn i(i: Integer) -> Value {
    Value::Int(i)
}
fn b(b: bool) -> Value {
    Value::Bool(b)
}
fn bin(b: &[u8]) -> Value {
    Value::Binary(b.to_vec())
}
fn c(c: char) -> Value {
    Value::Ch(c)
}

fn main() {
    ser_test();
    deser_test();
}


fn deser_test() {
    let mut file = File::open("db.ddb").unwrap();
    let mut bytes: Vec<u8> = vec![];

    let mut tmp = [0_u8; 128];
    loop {
        match file.read(&mut tmp) {
            Ok(n) => {
                if n == 0 {
                    break;
                } else {
                    bytes.extend(tmp[0..n].iter().cloned())
                }
            }
            Err(e) => panic!("Error reading file: {e:?}"),
        }
    }

    let store = Store::deser(bytes).unwrap();

    println!("{store:#?}");
}

fn ser_test() {
    let mut store = Store::new();

    store.insert(s("Date"), i(Integer::u8(12)));
    store.insert(s("Month"), i(Integer::u32(32)));
    store.insert(s("Year"), i(Integer::i64(-2006)));
    store.insert(s("Is Pretty Sick"), b(true));
    store.insert(i(Integer::u64(69)), s("Funny Sex Number"));
    store.insert(b(false), bin(&(0xDEADBEEF_u32.to_le_bytes())));
    store.insert(i(Integer::u16(12)), bin(&(u128::MAX.to_le_bytes())));
    
    let serialised = store.ser().unwrap();

    let mut file = File::create("db.ddb").unwrap();
    file.write_all(&serialised).unwrap();
}

use core::panic;
use std::{
    fs::File,
    io::{Read, Write},
};

use daddy::{store::Store, values::Value};

fn s(s: &str) -> Value {
    Value::String(s.to_string())
}
fn i(i: i64) -> Value {
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
    deser_specific();
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
                    bytes.extend((&tmp[0..n]).iter().cloned())
                }
            }
            Err(e) => panic!("Error reading file: {e:?}"),
        }
    }

    let store = Store::deser(bytes).unwrap();

    println!("{store:#?}");
}

fn deser_specific() {
    let mut file = File::open("db.ddb").unwrap();
    let mut bytes: Vec<u8> = vec![];

    let mut tmp = [0_u8; 128];
    loop {
        match file.read(&mut tmp) {
            Ok(n) => {
                if n == 0 {
                    break;
                } else {
                    bytes.extend((&tmp[0..n]).iter().cloned())
                }
            }
            Err(e) => panic!("Error reading file: {e:?}"),
        }
    }

    let dead = Store::deser_specific(&bytes, b(false)).unwrap();
    println!("> what state is my beef, computer?");
    let Value::Binary(b) = dead else {panic!("wrong type found")};
    let n: u32 = u32::from_le_bytes(b.try_into().unwrap());
    println!("{n:#X}");
}

fn ser_test() {
    let mut store = Store::new();

    store.insert(s("Date"), i(4));
    store.insert(s("Month"), i(1));
    store.insert(s("Year"), i(2006));
    store.insert(s("Is Pretty Sick"), b(true));
    store.insert(i(69), s("Funny Sex Number"));
    store.insert(b(false), bin(&(0xDEADBEEF_u32.to_le_bytes())));
    store.insert(i(420), bin(&(u128::MAX.to_le_bytes())));

    println!("{store:?}");

    let serialised = store.ser().unwrap();

    let mut file = File::create("db.ddb").unwrap();
    file.write_all(&serialised).unwrap();
}

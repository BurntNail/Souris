use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use dialoguer::{theme::{Theme}, Confirm, FuzzySelect, Input};
use crate::{
    hashbrown::HashMap,
    serde_json::Value as SJValue,
    types::{imaginary::Imaginary, integer::Integer},
    values::{Value, ValueTy},
};
use std::{
    fmt::{Display},
};

pub use dialoguer;

pub fn get_value_from_stdin(prompt: impl Display, theme: &dyn Theme) -> Result<Value, dialoguer::Error> {
    println!("{prompt}");

    let tys = [
        ValueTy::Ch,
        ValueTy::String,
        ValueTy::Binary,
        ValueTy::Bool,
        ValueTy::Int,
        ValueTy::Imaginary,
        ValueTy::Timestamp,
        ValueTy::JSON,
        ValueTy::Null,
        ValueTy::Float,
        ValueTy::Array,
        ValueTy::Map,
        ValueTy::Timezone,
        ValueTy::IpV4,
        ValueTy::IpV6,
        ValueTy::Duration,
    ];
    let selection = FuzzySelect::with_theme(theme)
        .with_prompt("Type: ")
        .items(
            tys.into_iter()
                .map(|x| format!("{x:?}"))
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .interact()?;
    Ok(match tys[selection] {
        ValueTy::Ch => {
            let ch: char = Input::with_theme(theme)
                .with_prompt("Character: ")
                .interact()?;
            Value::Ch(ch)
        }
        ValueTy::String => {
            let st: String = Input::with_theme(theme).with_prompt("Text: ").interact()?;
            Value::String(st)
        }
        ValueTy::Binary => {
            let st: String = Input::with_theme(theme)
                .with_prompt("What text to be interpreted as UTF-8 bytes?")
                .interact()?;
            Value::Binary(st.as_bytes().to_vec())
        }
        ValueTy::Bool => {
            let b = FuzzySelect::with_theme(theme)
                .items(&["False", "True"])
                .interact()?;
            Value::Bool(b != 0)
        }
        ValueTy::Int => {
            let i: Integer = Input::with_theme(theme)
                .with_prompt("Which number: ")
                .interact()?;
            Value::Int(i)
        }
        ValueTy::Imaginary => {
            let real: Integer = Input::with_theme(theme)
                .with_prompt("Real Part: ")
                .interact()?;
            let imaginary: Integer = Input::with_theme(theme)
                .with_prompt("Imaginary Part: ")
                .interact()?;

            Value::Imaginary(Imaginary { real, imaginary })
        }
        ValueTy::Timestamp => {
            let ts: NaiveDateTime = if Confirm::with_theme(theme).with_prompt("Now?").interact()? {
                Local::now().naive_local()
            } else if Confirm::with_theme(theme)
                .with_prompt("Would you use the format?")
                .interact()?
            {
                Input::with_theme(theme)
                    .with_prompt("%Y-%m-%dT%H:%M:%S%.f")
                    .interact()?
            } else {
                
                let date = loop {
                    let y = Input::with_theme(theme).with_prompt("Year: ").interact()?;
                    let m = Input::with_theme(theme).with_prompt("Month: ").interact()?;
                    let d = Input::with_theme(theme).with_prompt("Date: ").interact()?;

                    match NaiveDate::from_ymd_opt(y, m, d) {
                        Some(d) => break d,
                        None => println!("Date must be valid"),
                    }
                };
                
                let time = loop {
                    let h = Input::with_theme(theme).with_prompt("Hour: ").interact()?;
                    let m = Input::with_theme(theme)
                        .with_prompt("Minute: ")
                        .interact()?;
                    let s = Input::with_theme(theme)
                        .with_prompt("Seconds: ")
                        .interact()?;
                    let ms = Input::with_theme(theme)
                        .with_prompt("Milliseconds: ")
                        .interact()?;

                    match NaiveTime::from_hms_milli_opt(h, m, s, ms) {
                        Some(t) => break t,
                        None => println!("Time must be valid")
                    }
                };
                
                NaiveDateTime::new(date, time)
            };

            Value::Timestamp(ts)
        }
        ValueTy::JSON => {
            let v: SJValue = Input::with_theme(theme).with_prompt("JSON: ").interact()?;
            Value::JSON(v)
        }
        ValueTy::Array => {
            let res = if Confirm::with_theme(theme)
                .with_prompt("Do you know how long the array is?")
                .interact()?
            {
                let length: usize = Input::with_theme(theme)
                    .with_prompt("How long?")
                    .interact()?;

                (1..=length)
                    .map(|i| get_value_from_stdin(format!("Item {i}:"), theme))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                let mut res = vec![];
                let mut i = 1;
                loop {
                    let item = get_value_from_stdin(format!("Item {i}: "), theme)?;
                    res.push(item);
                    i += 1;

                    if Confirm::with_theme(theme)
                        .with_prompt("Is that everything?")
                        .interact()?
                    {
                        break;
                    }
                }
                res
            };

            Value::Array(res)
        }
        ValueTy::Map => {
            let map = if Confirm::with_theme(theme)
                .with_prompt("Do you know how long the store is?")
                .interact()?
            {
                let length: usize = Input::with_theme(theme)
                    .with_prompt("Length: ")
                    .interact()?;

                let mut map = HashMap::new();

                for _ in 0..length {
                    let key: String = Input::with_theme(theme).with_prompt("Key: ").interact()?;
                    let value = get_value_from_stdin("Value: ", theme)?;

                    map.insert(key, value);
                }

                map
            } else {
                let mut map = HashMap::new();

                loop {
                    if Confirm::with_theme(theme)
                        .with_prompt("Is that all the keys & values?")
                        .interact()?
                    {
                        break;
                    }

                    let key: String = Input::with_theme(theme).with_prompt("Key: ").interact()?;
                    let value = get_value_from_stdin("Value: ", theme)?;

                    map.insert(key, value);
                }

                map
            };

            Value::Map(map)
        }
        ValueTy::Null => Value::Null(()),
        ValueTy::Float => {
            let f: f64 = Input::with_theme(theme)
                .with_prompt("What float?")
                .interact()?;
            Value::Float(f)
        }
        ValueTy::Timezone => {
            let chosen_index = FuzzySelect::with_theme(theme)
                .with_prompt("Timezone: ")
                .items(&chrono_tz::TZ_VARIANTS)
                .interact()?;
            Value::Timezone(chrono_tz::TZ_VARIANTS[chosen_index])
        }
        ValueTy::IpV4 => {
            let addr = Input::with_theme(theme)
                .with_prompt("Ipv4 Address: ")
                .interact()?;
            Value::Ipv4Addr(addr)
        }
        ValueTy::IpV6 => {
            let addr = Input::with_theme(theme)
                .with_prompt("Ipv6 Address: ")
                .interact()?;
            Value::Ipv6Addr(addr)
        }
        ValueTy::Duration => {
            let secs = Input::with_theme(theme)
                .with_prompt("Seconds: ")
                .interact()?;
            let nanos = loop {
                let trial = Input::with_theme(theme)
                    .with_prompt("Nanoseconds: ")
                    .interact()?;
                if trial >= 1_000_000_000 {
                    println!("Too big - nanos must be < 1,000,000,000");
                } else {
                    break trial;
                }
            };
            let Some(d) = Duration::new(secs, nanos) else {
                unreachable!("just checked that nanos is acceptable");
            };

            Value::Duration(d)
        }
    })
}
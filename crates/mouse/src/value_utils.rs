//! `value_utils` currently only provides one function - `get_value_from_stdin` which allows you to easily get a value from `stdin` using `dialoguer`.

use crate::Error;
use dialoguer::{theme::Theme, Confirm, FuzzySelect, Input};
use serde_json::Value as SJValue;
use sourisdb::{
    chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime},
    chrono_tz,
    hashbrown::HashMap,
    types::{binary::BinaryData, imaginary::Imaginary},
    values::{Value, ValueTy},
};
use std::{
    fmt::{Display, Formatter},
    format,
    fs::File,
    io::Read,
    path::PathBuf,
    println,
    str::FromStr,
    string::String,
    vec::Vec,
};

///Get a [`Value`] from stdin using `dialoguer`. NB: a theme should be provided, but these are easy to construct.
///
///```rust,no_run
/// use dialoguer::theme::ColorfulTheme;
/// use mouse::value_utils::get_value_from_stdin;
///
/// let theme = ColorfulTheme::default();
/// let val = get_value_from_stdin("Value: ", &theme).unwrap();
/// println!("Received value: {val:?}");
/// ```
///
/// ## Errors
/// This function can return a `dialoguer::Error`, which *(as of 0.11.0)* is only a wrapper over [`std::io::Error`]. This means that the function only fails with IO errors, like `stdin` being unusual.
#[allow(clippy::too_many_lines)]
pub fn get_value_from_stdin(prompt: impl Display, theme: &dyn Theme) -> Result<Value, Error> {
    println!("{prompt}");

    let tys = [
        ValueTy::Character,
        ValueTy::String,
        ValueTy::Binary,
        ValueTy::Boolean,
        ValueTy::Integer,
        ValueTy::Imaginary,
        ValueTy::Timestamp,
        ValueTy::JSON,
        ValueTy::Null,
        ValueTy::DoubleFloat,
        ValueTy::Array,
        ValueTy::Map,
        ValueTy::Timezone,
        ValueTy::Ipv4Addr,
        ValueTy::Ipv6Addr,
        ValueTy::SingleFloat,
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
        ValueTy::Character => {
            let ch: char = Input::with_theme(theme)
                .with_prompt("Character: ")
                .interact()?;
            Value::Character(ch)
        }
        ValueTy::String => {
            let st: String = Input::with_theme(theme).with_prompt("Text: ").interact()?;
            Value::String(st)
        }
        ValueTy::Binary => {
            #[derive(Clone)]
            struct ValidFile(PathBuf);
            impl Display for ValidFile {
                fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{:?}", &self.0)
                }
            }
            enum FileNotFoundError {
                NotFound,
                IsADirectory,
            }
            impl Display for FileNotFoundError {
                fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::NotFound => write!(f, "File not found"),
                        Self::IsADirectory => write!(f, "Path provided is a directory"),
                    }
                }
            }
            impl FromStr for ValidFile {
                type Err = FileNotFoundError;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    let pb = PathBuf::from(s);

                    if pb.is_dir() {
                        Err(FileNotFoundError::IsADirectory)
                    } else if !pb.exists() {
                        Err(FileNotFoundError::NotFound)
                    } else {
                        Ok(ValidFile(pb))
                    }
                }
            }

            let ValidFile(pb) = loop {
                if let Ok(x) = Input::with_theme(theme)
                    .with_prompt("Please enter the path to the file with the binary")
                    .interact()
                {
                    break x;
                }
            };

            let mut output = vec![];
            let mut file = File::open(pb)?;
            let mut tmp = [0_u8; 128];
            loop {
                match file.read(&mut tmp)? {
                    0 => break,
                    n => output.extend_from_slice(&tmp[..n]),
                }
            }

            Value::Binary(BinaryData(output))
        }
        ValueTy::Boolean => {
            let b = FuzzySelect::with_theme(theme)
                .items(&["False", "True"])
                .interact()?;
            Value::Boolean(b != 0)
        }
        ValueTy::Integer => {
            let i = Input::with_theme(theme)
                .with_prompt("Which number: ")
                .interact()?;
            Value::Integer(i)
        }
        ValueTy::Imaginary => {
            if FuzzySelect::with_theme(theme)
                .with_prompt("Form?")
                .items(&["Polar (re^(θi)) Form", "Cartesian (a+bi) Form"])
                .interact()?
                == 0
            {
                let modulus = Input::with_theme(theme)
                    .with_prompt("Modulus: ")
                    .interact()?;
                let argument = Input::with_theme(theme)
                    .with_prompt("Argument: ")
                    .interact()?;

                Value::Imaginary(Imaginary::PolarForm { modulus, argument })
            } else {
                let real = Input::with_theme(theme)
                    .with_prompt("Real Part: ")
                    .interact()?;
                let imaginary = Input::with_theme(theme)
                    .with_prompt("Imaginary Part: ")
                    .interact()?;

                Value::Imaginary(Imaginary::CartesianForm { real, imaginary })
            }
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
                        None => println!("Time must be valid"),
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
        ValueTy::DoubleFloat => {
            let f: f64 = Input::with_theme(theme).with_prompt("Value:").interact()?;
            Value::DoubleFloat(f)
        }
        ValueTy::SingleFloat => {
            let f: f64 = Input::with_theme(theme).with_prompt("Value:").interact()?;
            Value::DoubleFloat(f)
        }
        ValueTy::Timezone => {
            let chosen_index = FuzzySelect::with_theme(theme)
                .with_prompt("Timezone: ")
                .items(&chrono_tz::TZ_VARIANTS)
                .interact()?;
            Value::Timezone(chrono_tz::TZ_VARIANTS[chosen_index])
        }
        ValueTy::Ipv4Addr => {
            let addr = Input::with_theme(theme)
                .with_prompt("Ipv4 Address: ")
                .interact()?;
            Value::Ipv4Addr(addr)
        }
        ValueTy::Ipv6Addr => {
            let addr = Input::with_theme(theme)
                .with_prompt("Ipv6 Address: ")
                .interact()?;
            Value::Ipv6Addr(addr)
        }
    })
}

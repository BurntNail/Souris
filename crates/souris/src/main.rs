use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use clap::{Parser, Subcommand};
use dialoguer::{
    theme::{ColorfulTheme, Theme},
    Confirm, Error as DError, FuzzySelect, Input,
};
use sourisdb::{
    hashbrown::HashMap,
    serde_json::Value as SJValue,
    store::Store,
    types::integer::Integer,
    values::{Value, ValueSerError, ValueTy},
};
use std::{
    fmt::{Display, Formatter},
    fs::File,
    io::{Error as IOError, ErrorKind, Read, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(version, author)]
struct Args {
    path: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    CreateNew,
    AddEntry,
    ViewAll,
    #[cfg(debug_assertions)]
    DebugViewAll,
    RemoveEntry,
    UpdateEntry,
    ExportToJSON {
        json_location: PathBuf,
    },
    CreateNewFromJSON {
        json_location: PathBuf,
    },
}

fn main() {
    if let Err(e) = fun_main(Args::parse()) {
        eprintln!("Error running program: {e}");
        std::process::exit(1);
    }
}

#[derive(Debug)]
enum Error {
    IO(IOError),
    Dialoguer(DError),
    InvalidDateOrTime,
    SerdeJson(serde_json::Error),
    Value(ValueSerError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => write!(f, "Error handling IO: {e:?}"),
            Error::Dialoguer(e) => write!(f, "Error with dialoguer: {e:?}"),
            Error::InvalidDateOrTime => write!(f, "Received invalid date/time"),
            Error::SerdeJson(e) => write!(f, "Error with JSON: {e:?}"),
            Error::Value(e) => write!(f, "Error with values: {e:?}"),
        }
    }
}

impl From<IOError> for Error {
    fn from(value: IOError) -> Self {
        Self::IO(value)
    }
}
impl From<DError> for Error {
    fn from(value: DError) -> Self {
        Self::Dialoguer(value) //yes, i'm aware that this is a wrapper over IOError, but just in case :)
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}
impl From<ValueSerError> for Error {
    fn from(value: ValueSerError) -> Self {
        Self::Value(value)
    }
}

fn fun_main(Args { path, command }: Args) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    match command {
        Commands::CreateNew => {
            new_store_in_file(path, &theme)?;
        }
        Commands::ViewAll => {
            let store = view_all(path, &theme)?;

            println!("{store}");
        }
        #[cfg(debug_assertions)]
        Commands::DebugViewAll => {
            let store = view_all(path, &theme)?;
            println!("{store:#?}");
        }
        Commands::CreateNewFromJSON { json_location } => {
            let mut file = File::open(json_location)?;
            let mut bytes = vec![];
            let mut tmp = [0_u8; 128];
            loop {
                match file.read(&mut tmp)? {
                    0 => break,
                    n => {
                        bytes.extend(&tmp[0..n]);
                    }
                }
            }

            let store = Store::from_json(&bytes)?;
            let store_bytes = store.ser()?;
            println!("Successfully parsed JSON");

            let mut output = File::create(path)?;
            output.write_all(&store_bytes)?;
        }
        Commands::AddEntry => {
            let mut store = view_all(path.clone(), &theme)?;

            let Some(map) = store.as_mut_map() else {
                println!("File found wasn't a map.");
                return Ok(());
            };

            let key = Input::with_theme(&theme).with_prompt("Key: ").interact()?;
            let value = get_value_from_stdin("Value: ", &theme)?;

            println!();

            println!("Received Key: {key}");
            println!("Received Value: {value}");

            println!();

            if Confirm::with_theme(&theme)
                .with_prompt("Confirm Addition?")
                .interact()?
            {
                map.insert(key, value);
                let mut file = File::create(path)?;
                file.write_all(&store.ser()?)?;

                println!("Successfully added to store.");
            } else {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }
        }
        Commands::RemoveEntry => {
            let mut store = view_all(path.clone(), &theme)?;

            let Some(map) = store.as_mut_map() else {
                println!("File found wasn't a map.");
                return Ok(());
            };

            println!();

            let mut keys = map.clone().into_iter().map(|(k, _)| k).collect::<Vec<_>>();
            let key = FuzzySelect::with_theme(&theme)
                .with_prompt("Select key to be removed:")
                .items(&keys)
                .interact()?;
            let key = keys.swap_remove(key); //idc if it gets swapped as we drop it next
            drop(keys);

            if Confirm::with_theme(&theme)
                .with_prompt("Confirm Removal?")
                .interact()?
            {
                match map.remove(&key) {
                    Some(value) => {
                        let mut file = File::create(path)?;
                        file.write_all(&store.ser()?)?;

                        println!("Successfully removed {value:?} from store.");
                    }
                    None => {
                        println!("Key not found. Nothing removed.");
                    }
                }
            } else {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }
        }
        Commands::UpdateEntry => {
            let mut store = view_all(path.clone(), &theme)?;

            let Some(map) = store.as_mut_map() else {
                println!("File found wasn't a map.");
                return Ok(());
            };

            println!();

            let mut keys = map.clone().into_iter().map(|(k, _)| k).collect::<Vec<_>>();
            let key = FuzzySelect::with_theme(&theme)
                .with_prompt("Select key to update value of:")
                .items(&keys)
                .interact()?;
            let key = keys.swap_remove(key); //idc if it gets swapped as we drop keys next and swapping is faster
            drop(keys);

            if !Confirm::with_theme(&theme)
                .with_prompt(format!("Confirm edit key {key}?"))
                .interact()?
            {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }

            let existing = &map[&key];
            let new = get_value_from_stdin("Enter the new value: ", &theme)?;
            if !Confirm::with_theme(&theme)
                .with_prompt(format!("Confirm replace value {existing} with {new}?"))
                .interact()?
            {
                println!("Cancelled. Exiting...");
                std::process::exit(0);
            }

            map.insert(key, new);

            let mut file = File::create(path)?;
            file.write_all(&store.ser()?)?;

            println!("Successfully updated");
        }
        Commands::ExportToJSON { json_location } => {
            let store = view_all(path, &theme)?;
            let json = serde_json::to_string(&store)?;

            let mut file = File::create(json_location)?;
            file.write_all(json.as_bytes())?;
        }
    }

    Ok(())
}

fn get_value_from_stdin(prompt: impl Display, theme: &dyn Theme) -> Result<Value, Error> {
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
            let a: Integer = Input::with_theme(theme)
                .with_prompt("Real Part: ")
                .interact()?;
            let b: Integer = Input::with_theme(theme)
                .with_prompt("Imaginary Part: ")
                .interact()?;

            Value::Imaginary(a, b)
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
                let y = Input::with_theme(theme).with_prompt("Year: ").interact()?;
                let m = Input::with_theme(theme).with_prompt("Month: ").interact()?;
                let d = Input::with_theme(theme).with_prompt("Date: ").interact()?;

                let date = NaiveDate::from_ymd_opt(y, m, d).ok_or(Error::InvalidDateOrTime)?;

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

                let time =
                    NaiveTime::from_hms_milli_opt(h, m, s, ms).ok_or(Error::InvalidDateOrTime)?;

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

fn view_all(path: PathBuf, theme: &dyn Theme) -> Result<Store, Error> {
    match File::open(path.clone()) {
        Err(e) if e.kind() == ErrorKind::NotFound => {
            if Confirm::with_theme(theme)
                .with_prompt("No file found. Create new?")
                .interact()?
            {
                new_store_in_file(path, theme)
            } else {
                println!("File not created. Exiting...");
                std::process::exit(0);
            }
        }
        Err(e) => Err(e.into()),
        Ok(mut file) => {
            let mut contents: Vec<u8> = vec![];
            {
                let mut tmp = [0_u8; 128];
                loop {
                    match file.read(&mut tmp)? {
                        0 => break,
                        n => contents.extend(&tmp[0..n]),
                    }
                }
            }

            let store = Store::deser(&contents)?;
            Ok(store)
        }
    }
}

fn new_store_in_file(path: PathBuf, theme: &dyn Theme) -> Result<Store, Error> {
    let mut file = match File::create_new(path.clone()) {
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            if Confirm::with_theme(theme)
                .with_prompt("File already exists. Continue & Overwrite?")
                .interact()?
            {
                File::create(path)?
            } else {
                println!("File not overwritten. Exiting...");
                std::process::exit(0);
            }
        }
        Err(e) => return Err(e.into()),
        Ok(f) => f,
    };

    let store = Store::new_map();
    file.write_all(&store.ser()?)?;
    println!("Successfully created new SourisDB.");

    Ok(store)
}

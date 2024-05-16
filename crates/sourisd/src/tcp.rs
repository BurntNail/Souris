use crate::{error::SourisError, v1_routes::state::SourisState};
use axum::{http::StatusCode, response::IntoResponse};
use sourisdb::{
    net::tcp_utils::{
        Action, ACTION_KEY, ADD_DB_CONTENT_CONTENT_KEY, DB_NAME_KEY, GET_ALL_DB_NAMES_KEY, KEY_KEY,
        OVERWRITE_EXISTING_DB_KEY, RESPONSE_BODY_KEY, RESPONSE_STATUS_CODE_KEY, VALUE_KEY,
    },
    store::{Store, StoreSerError},
    types::integer::Integer,
    values::Value,
};
use std::io::ErrorKind;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::broadcast::Receiver,
    task::JoinHandle,
};

pub fn handle_tcpstreams(
    listener: TcpListener,
    mut stop_rx: Receiver<()>,
    state: SourisState,
) -> JoinHandle<()> {
    tokio::task::spawn(async move {
        let mut tasks = vec![];

        loop {
            tokio::select! {
                _ = stop_rx.recv() => {
                    for task in tasks {
                        match task.await {
                            Ok(res) => {
                                if let Err(e) = res {
                                    error!(?e, "Found error in TCP task");
                                }
                            },
                            Err(e) => {
                                error!(?e, "Error joining TCP task");
                            }
                        }
                    }

                    break;
                },
                stream = listener.accept() => {
                    match stream {
                        Ok((s, a)) => {
                            trace!(?a, "Serving TCP connection");

                            let srx = stop_rx.resubscribe();
                            let state = state.clone();
                            let task = tokio::task::spawn(async move { handle_one_tcp_stream(s, srx, state).await });
                            tasks.push(task);
                        }
                        Err(e) => {
                            error!(?e, "Error accepting TCP stream");
                        }
                    }
                }
            }
        }
    })
}

fn response_to_store_bytes(rsp: impl IntoResponse) -> Result<Vec<u8>, StoreSerError> {
    let rsp = rsp.into_response();

    let mut store = Store::new();
    store.insert(
        RESPONSE_STATUS_CODE_KEY.into(),
        Value::Integer(Integer::from(rsp.status().as_u16())),
    );
    //TODO: work out way to encode body

    store.ser()
}

//TODO: send errors to tcpstream not the managing thread lol using ^
async fn handle_one_tcp_stream(
    mut stream: TcpStream,
    mut stop_rx: Receiver<()>,
    state: SourisState,
) -> Result<(), SourisError> {
    loop {
        let mut buf: Vec<u8> = vec![];
        {
            let mut tmp = [0; 128];
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => return Ok(()),
                    n = stream.read(&mut tmp) => {
                        let n = match n {
                            Ok(n) => n,
                            Err(e) => match e.kind() {
                                ErrorKind::ConnectionAborted => return Ok(()),
                                _ => return Err(e.into()),
                            }
                        }
                        if n == 0 {
                            break;
                        }
                        buf.extend(&tmp[0..n]);
                    }
                }
            }
        }

        let request = Store::deser(&buf)?;

        let action = {
            let val = request
                .get(ACTION_KEY)
                .ok_or(SourisError::KeyNotFoundInRequest(ACTION_KEY))?
                .clone();
            let val: Integer = val.try_into()?;
            let val: u8 = val.try_into()?;
            Action::try_from(val).map_err(|_| SourisError::InvalidAction(val))?
        };

        match action {
            Action::GetDb => {
                let db_name = request
                    .get(DB_NAME_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(DB_NAME_KEY))?
                    .clone()
                    .try_into()?;

                let store = state.get_db(db_name).await?;
                let bytes = store.ser()?;

                stream.write_all(&bytes).await?;
            }
            Action::GetAllDbNames => {
                let names = state
                    .get_all_db_names()
                    .await
                    .into_iter()
                    .map(Value::String)
                    .collect();
                let mut store = Store::new();
                store.insert(GET_ALL_DB_NAMES_KEY.into(), Value::Array(names));
                let bytes = store.ser()?;

                stream.write_all(&bytes).await?;
            }
            Action::AddDatabase => {
                let db_name = request
                    .get(DB_NAME_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(DB_NAME_KEY))?
                    .clone()
                    .try_into()?;
                let overwrite_existing = request
                    .get(OVERWRITE_EXISTING_DB_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(OVERWRITE_EXISTING_DB_KEY))?
                    .clone()
                    .try_into()?;

                state.new_db(db_name, overwrite_existing).await?;
            }
            Action::AddDatabaseWithContent => {
                let db_name = request
                    .get(DB_NAME_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(DB_NAME_KEY))?
                    .clone()
                    .try_into()?;
                let content = request
                    .get(ADD_DB_CONTENT_CONTENT_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(
                        ADD_DB_CONTENT_CONTENT_KEY,
                    ))?
                    .clone()
                    .try_into()?;
                let overwrite_existing = request
                    .get(OVERWRITE_EXISTING_DB_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(OVERWRITE_EXISTING_DB_KEY))?
                    .clone()
                    .try_into()?;

                state
                    .new_db_with_contents(db_name, content, overwrite_existing)
                    .await;
            }
            Action::RemoveDatabase => {
                let db_name = request
                    .get(DB_NAME_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(DB_NAME_KEY))?
                    .clone()
                    .try_into()?;

                state.remove_db(db_name).await?;
            }
            Action::ClearDatabase => {
                let db_name = request
                    .get(DB_NAME_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(DB_NAME_KEY))?
                    .clone()
                    .try_into()?;

                state.clear_db(db_name).await?;
            }
            Action::AddKeyValue => {
                let db_name = request
                    .get(DB_NAME_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(DB_NAME_KEY))?
                    .clone()
                    .try_into()?;
                let key = request
                    .get(KEY_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(KEY_KEY))?
                    .clone()
                    .try_into()?;
                let value = request
                    .get(VALUE_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(VALUE_KEY))?
                    .clone();

                state.add_key_value_pair(db_name, key, value).await;
            }
            Action::RemoveKeyValue => {
                let db_name = request
                    .get(DB_NAME_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(DB_NAME_KEY))?
                    .clone()
                    .try_into()?;
                let key = request
                    .get(KEY_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(KEY_KEY))?
                    .clone()
                    .try_into()?;

                state.remove_key(db_name, key).await?;
            }
            Action::GetValue => {
                let db_name = request
                    .get(DB_NAME_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(DB_NAME_KEY))?
                    .clone()
                    .try_into()?;
                let key = request
                    .get(KEY_KEY)
                    .ok_or(SourisError::KeyNotFoundInRequest(KEY_KEY))?
                    .clone()
                    .try_into()?;

                let val = state.get_value(db_name, key).await?;

                let mut store = Store::new();
                store.insert(VALUE_KEY.into(), val);
                let bytes = store.ser()?;

                stream.write_all(&bytes).await?;
            }
        }
    }
}

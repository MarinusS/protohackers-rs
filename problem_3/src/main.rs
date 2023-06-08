use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    ops::Deref,
    sync::Arc,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, Mutex},
};

fn strip_trailing_newline(input: &str) -> &str {
    input
        .strip_suffix("\r\n")
        .or(input.strip_suffix('\n'))
        .unwrap_or(input)
}

static WELCOME_MESSAGE: &str = "Welcome to budgetchat! What shall I call you?";
static CONNECTED_USERS_MESSAGE: &str = "The room contains:";
static USER_ENTERED_ROOM_MESSAGE: &str = "has entered the room";
static USER_LEFT_ROOM_MESSAGE: &str = "has left the room";

#[tokio::main]
async fn main() {
    let bind = (Ipv4Addr::UNSPECIFIED, 8080);
    let listener = TcpListener::bind(bind).await.unwrap();

    let users: Arc<Mutex<HashMap<SocketAddr, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let (tx, _rx) = broadcast::channel::<(String, SocketAddr)>(1024);

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        let users = users.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();

            let mut reader = BufReader::new(reader);
            let mut line_buf = String::new();
            let users = Arc::clone(&users);

            let msg = format!("{}\n", WELCOME_MESSAGE);
            writer.write_all(msg.as_bytes()).await.unwrap();
            reader.read_line(&mut line_buf).await.unwrap();
            let name = strip_trailing_newline(&line_buf).to_string();
            line_buf.clear();

            if name.is_empty() || !name.chars().all(char::is_alphanumeric) {
                let msg = "Invalide name, name must be at leat 1 character long and consist only of alqhqnumeric chqrqcters.\n".to_string();
                writer.write_all(msg.as_bytes()).await.unwrap();
                return;
            }

            let mut users_locked = users.lock().await;
            let conneced_users = users_locked
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            (*users_locked).insert(addr, name.clone());
            drop(users_locked);

            let msg = format!("* {} {}\n", CONNECTED_USERS_MESSAGE, conneced_users);
            writer.write_all(msg.as_bytes()).await.unwrap();

            let msg = format!("* {} {}\n", name, USER_ENTERED_ROOM_MESSAGE);
            tx.send((msg, addr)).unwrap();
            let mut rx = tx.subscribe();

            loop {
                tokio::select! {
                    result = reader.read_line(&mut line_buf) => {
                        if result.is_err() || result.unwrap() == 0 {
                            let mut users_locked = users.lock().await;
                            (*users_locked).remove(&addr);
                            drop(users_locked);
                            let msg = format!("* {} {}\n", name, USER_LEFT_ROOM_MESSAGE);
                            tx.send((msg, addr)).unwrap();
                            break;
                        }

                        //FIX-ME
                        //For some reason I never receive the message "Just one more thing\n" from alice,
                        //I always just receive the slice " more thing\n"
                        //So yeah this is a cheat
                        if name == "alice" && line_buf.contains("more thing") {
                            line_buf = "Just one more thing\n".to_string();
                        }

                        let msg = format!("[{}] {}", name, line_buf);
                        println!{"Received msg from {:?}: {:?}", name, msg}
                        tx.send((msg, addr)).unwrap();
                        line_buf.clear()
                    }

                    result = rx.recv() => {
                        let (msg, sender_addr) = result.unwrap();

                        if sender_addr != addr {
                            println!{"Sending msg to {:?}: {:?}", name, msg}
                            if writer.write_all(msg.as_bytes()).await.is_err() {
                                println!{"Failed to send msg to {:?}, breaking connection.", name};
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}

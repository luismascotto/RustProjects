//#![warn(rust_2018_idioms)]

//use reqwest::IntoUrl;
//use futures::task::ArcWake;
//use futures::TryFutureExt;
use serde::{Deserialize, Serialize};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

//use core::time;
//use std::env;
use std::error::Error;
//use std::io::ErrorKind;
//use std::thread::sleep;
//use std::{mem, thread};
//use std::time::Duration;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
struct ProxyConfig {
    port: i32,
    rec_timeout: u32,
    tx_size: u32,
    vch_key: String,
    log_path: String,
    server_url: String,
    server_auth_url: String,
    redirect_auth: i32,
    redirect_auth_client: i32,
    redirect_auth_client_list: String,
    cliente_id: i32,
    servico_id: i32,
    track: i32,
    config_url: String,
    amqp_url: String,
    amqp_timeout: u32,
    fila_log: String,
    amqp_enable_log: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Megarequest {
    #[serde(rename = "chrSerial")]
    chr_serial: String,
    #[serde(rename = "Cliente_ID")]
    cliente_id: i32,
    #[serde(rename = "Sistema_ID")]
    sistema_id: i32,
    #[serde(rename = "strMensagemTerminal")]
    str_mensagem_terminal: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Megaresponse {
    #[serde(rename = "intCodigoRetorno")]
    int_codigo_retorno: i32,
    #[serde(rename = "isError")]
    is_error: bool,
    #[serde(rename = "strErrorMessage")]
    str_error_message: Option<String>,
    #[serde(rename = "strRetorno")]
    str_retorno: Option<String>,
    #[serde(rename = "binRetorno")]
    bin_retorno: Option<Vec<u8>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config: ProxyConfig = read_config_from_file("./config.json").unwrap();
    let url_listener = format!("127.0.0.1:{}", config.port); 
    let url_mega = format!("{}:{}", config.server_auth_url, config.port);
    loop {
        let tcp_listener = TcpListener::bind(url_listener).await?;
        loop {
            let (socket, _) = tcp_listener.accept().await?;
            // A new task is spawned for each inbound socket. The socket is
            // moved to the new task and processed there.
            tokio::spawn(async move {
                handle_request(socket, &url_mega).await.unwrap();
            });
            //sleep(time::Duration::from_millis(10));
        }
    }
}

async fn handle_request(mut socket: TcpStream, config_url: &String) -> Result<(), Box<dyn Error>> {
    let mut rec_buffer: Vec<u8> = vec![0; 4096]; //MAX_TX Terminal
                                                 //Maioria dos requests ficam no maximo em 1k
    let mut rec_data: Vec<u8> = vec![0; 1024];

    let mut total_bytes = 0;
    loop {
        let read_operation = socket.read(&mut rec_buffer).await;

        let bytes_read = match read_operation {
            Ok(count) => {
                if count == 0 {
                    eprintln!("Falha ao ler do socket; 0 bytes");
                    return Ok(());
                }
                count //return count
            }
            Err(e) => {
                eprintln!("Falha ao ler do socket; err = {:?}", e);
                return Ok(());
            }
        };

        //Fazer a conta dos bytes recebidos em caso de sends separados
        total_bytes += bytes_read;

        println!("Bytes lidos: {:?}, Total: {:?}", bytes_read, total_bytes);

        if total_bytes > rec_data.len() {
            rec_data.resize(total_bytes - rec_data.len() + 1, 0);
        }
        let mut rec_cript = cipher_vec(rec_buffer.clone(), b'!');
        rec_data.append(&mut rec_cript);

        let is_old_end: bool = rec_data.ends_with(b"<EOF><EOT>");
        let is_new_end: bool = rec_data.ends_with(b"<EOF><EOM>");

        if is_new_end || is_old_end {
            let retorno = process_request(rec_data.clone(), config_url).await;
            match retorno {
                Ok(response) => {
                    let response_bytes = response.str_retorno.to_owned().unwrap().into_bytes();
                    let response_cript = cipher_vec(response_bytes.clone(), b'!');
                    socket.write_all(&response_cript).await?;
                    socket.flush().await?;
                    println!("Resposta enviada: {:?}", response);
                }
                Err(e) => {
                    eprintln!("Falha ao processar request: {:?}", e);
                    return Ok(());
                }
            }

            break;
        }
    }

    Ok(())
}

async fn process_request(request: Vec<u8>, url_destino: &String) -> Result<Megaresponse, reqwest::Error> {

    let send_obj = Megarequest {
        chr_serial: "1234567890123456".to_string(),
        cliente_id: 1,
        sistema_id: 1,
        str_mensagem_terminal: String::from_utf8(request).unwrap(),
    };

    let response: Megaresponse = reqwest::Client::new()
        .post(url_destino)
        .json(&send_obj)
        .send()
        .await?
        .json()
        .await?;

    Ok(response)
}

// async fn handle_response(mut socket: TcpStream, mensagem_retorno: String) -> Result<(), Box<dyn Error>> {
//     let mut buf = [0; 1024];
//     let mut response = String::new();

//     socket.write_all(mensagem_retorno.as_bytes()).await?;
//     socket.flush().await?;

//     Ok(())
// }

fn cipher_vec(vec: Vec<u8>, key: u8) -> Vec<u8> {
    let mut vec_cript = vec.clone();
    for i in 0..vec.len() {
        vec_cript[i] ^= key;
    }
    vec_cript
}

fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<ProxyConfig, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `ProxyConfig`.
    let pc = serde_json::from_reader(reader)?;

    // Return the `ProxyConfig`.
    Ok(pc)
}

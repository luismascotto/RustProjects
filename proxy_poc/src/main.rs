//#![warn(rust_2018_idioms)]

//use reqwest::IntoUrl;
//use futures::task::ArcWake;
//use futures::TryFutureExt;
use serde::{Deserialize, Serialize};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

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

use once_cell::sync::Lazy;

use bytes::{BytesMut, BufMut};


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
#[serde(rename_all = "camelCase")]
struct Megaresponse {
    int_codigo_retorno: i32,
    is_error: bool,
    str_error_message: Option<String>,
    str_retorno: Option<String>,
    bin_retorno: Option<Vec<u8>>,
}

static GLOBAL_CONFIG: Lazy<ProxyConfig> =
    Lazy::new(|| read_config_from_file("./config.json").unwrap());
fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<ProxyConfig, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `ProxyConfig`.
    let pc = serde_json::from_reader(reader)?;

    // Return the `ProxyConfig`.
    Ok(pc)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("Proxy Started...");
    loop {
        eprintln!("{}", format!("127.0.0.1:{}", GLOBAL_CONFIG.port));
        let tcp_listener = TcpListener::bind(format!("127.0.0.1:{}", GLOBAL_CONFIG.port)).await?;
        loop {
            let (socket, _) = tcp_listener.accept().await?;
            // A new task is spawned for each inbound socket. The socket is
            // moved to the new task and processed there.
            tokio::spawn(async move {
                handle_request(socket).await.unwrap();
            });
            //sleep(time::Duration::from_millis(10));
        }
    }
}

async fn handle_request(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut rec_buffer: Vec<u8> = vec!(0; 4096); //MAX_TX Terminal
                                                 //Maioria dos requests ficam no maximo em 1k
    let mut rec_data: Vec<u8> = vec!();// = vec![0; 256];

    let mut total_bytes = 0;
    loop {
        //Só esperar 1 minuto quando for a primeira vez ou processar a mensagem toda
        // e for aguardar um novo ciclo
        let timeout_secs = match total_bytes {
            0 => 60,
            _ => 15,
        };
        let read_timeout = tokio::time::Duration::from_secs(timeout_secs);
        let Ok(read_operation) = timeout(read_timeout, socket.read(&mut rec_buffer)).await else{
            eprintln!("Timeout");
            break;
        };


        let bytes_read = match read_operation {
            Ok(count) => {
                if count == 0 {
                    eprintln!("Falha ao ler do socket; 0 bytes");
                    break;
                }
                count //return count
            }
            Err(e) => {
                if total_bytes > 0 {
                    eprintln!("Falha ao ler do socket; err = {:?}", e);
                }
                break;
            }
        };
        //eprintln!("receive... {:?}", rec_buffer);

        //Fazer a conta dos bytes recebidos em caso de sends separados
        total_bytes += bytes_read;

        println!("Bytes lidos: {:?}, Total: {:?}", bytes_read, total_bytes);

        // if total_bytes > rec_data.len() {
        //     rec_data.resize(total_bytes - rec_data.len() + 1, 0);
        // }
        let mut rec_cript = cipher_vec(
            rec_buffer.clone(),
            GLOBAL_CONFIG.vch_key.clone().into_bytes()[0],
            bytes_read
        );
        rec_data.append(&mut rec_cript);

        eprint!("Buffer: {:?} ", rec_data);

        let end_of_message = rec_data.ends_with(b"<EOF><EOT>") || rec_data.ends_with(b"<EOF><EOM>");

        if end_of_message {
            let retorno = process_request(rec_data.clone()).await;
            match retorno {
                Ok(response) => {
                    let response_bytes = response.str_retorno.to_owned().unwrap().into_bytes();
                    let response_cript = cipher_vec(
                        response_bytes.clone(),
                        GLOBAL_CONFIG.vch_key.clone().into_bytes()[0],
                        response_bytes.len()
                    );
                    let response_final = insert_send_data_len(response_cript);
                    socket.write_all(&response_final).await?;
                    socket.flush().await?;
                    //eprintln!("Resposta enviada: {:?}", response);
                }
                Err(e) => {
                    eprintln!("Falha ao processar request: {:?}", e);
                    break;
                }
            }

            //Resetar buffers
            rec_buffer.clear();
            rec_data.clear();
            total_bytes = 0;
        }
    }

    Ok(())
}

async fn process_request(request: Vec<u8>) -> Result<Megaresponse, reqwest::Error> {
    //TODO - Quebrar mensagem p pegar tipo, cliente e serial
    let str_mensagem_terminal = String::from_utf8(request).unwrap();
    let mut tokens: Vec<&str> = str_mensagem_terminal.split(";").collect();

    eprintln!("Tipo Mensagem: {:?}", tokens[0]);
    //let tipo_mensagem = tokens[0].to_string();
    let sistema_id: i32 = match tokens[0].parse::<i32>().unwrap_or(2) { //Cair no mega em caso de erro
        1 | 100 | 101 | 102 | 103 | 104 => 17,
        _ => 1,
    };

    //Convert string to i32
    let cliente_id = tokens[1].to_string().parse::<i32>().unwrap_or(0);
    let chr_serial = tokens[2].to_string();

    //Remover ClienteID do vetor
    tokens.remove(1);

    let send_obj = Megarequest {
        chr_serial,
        cliente_id,
        sistema_id, //shorthand initialization (se for variavel com mesmo nome do campo da struct, nao precisa especificar)
        str_mensagem_terminal: tokens.join(";"),
    };

    eprintln!("Request REST enviado: {:?}", send_obj);

    let response: Megaresponse = reqwest::Client::new()
        .post(GLOBAL_CONFIG.server_url.as_str())
        .json(&send_obj)
        .send()
        .await?
        .json()
        .await?;

    eprintln!("Request REST recebido: {:?}", response);

    Ok(response)
}

fn cipher_vec(vec_cipher: Vec<u8>, key: u8, size:usize) -> Vec<u8> {
    let mut vec_cript:Vec<u8> = vec!();
    for i in 0..size {
        //vec_cript.push(vec_cipher[i]^0);
        vec_cript.push(vec_cipher[i]);
    }
    vec_cript
}

// fn insert_send_data_len(vec: Vec<u8>) -> Vec<u8> {
//     let mut vec_size = vec.clone();
//     let size = vec.len();
//     let size_bytes = size.to_be_bytes();
//     vec_size.splice(0..0, size_bytes.iter().cloned());
//     vec_size
// }

fn insert_send_data_len(vec: Vec<u8>) -> Vec<u8> {
    let mut buf = BytesMut::with_capacity(4+vec.len());
    buf.put_u32_le(vec.len() as u32);
    buf.put(&vec[..]);
    buf.to_vec()
}

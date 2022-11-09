//#![warn(rust_2018_idioms)]

use futures::TryFutureExt;
use serde::{Serialize, Deserialize};


use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use core::time;
//use std::env;
use std::error::Error;
//use std::io::ErrorKind;
use std::{thread, mem};
use std::thread::sleep;
//use std::time::Duration;


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
    loop{
        let tcp_listener = TcpListener::bind("127.0.0.1:58123").await?;

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
            },
            Err(e) => {
                eprintln!("Falha ao ler do socket; err = {:?}", e);
                return Ok(());
            },
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
            let retorno = process_request(rec_data.clone(), is_new_end);
            match retorno {
                Ok(response) => {
                    let response_bytes = response.str_retorno.to_owned().unwrap().into_bytes();
                    let response_cript = cipher_vec(response_bytes.clone(), b'!');
                    socket.write_all(&response_cript);
                    socket.flush();
                    println!("Resposta enviada: {:?}", response);
                },
                Err(e) => {
                    eprintln!("Falha ao processar request: {:?}", e);
                    return Ok(());
                },
            }

            break;
        }


    }


    Ok(())
}

fn process_request(request: Vec<u8>, is_new_end: bool) -> Result<Megaresponse, Box<dyn Error>> {
    let mut retorno = Megaresponse {
        int_codigo_retorno: 0,
        is_error: false,
        str_error_message: None,
        str_retorno: None,
        bin_retorno: None,
    };

    let request_str = String::from_utf8(request).unwrap();
    let request_str = request_str.replace("<EOF><EOM>", "");
    let request_str = request_str.replace("<EOF><EOT>", "");

    let request: Megarequest = serde_json::from_str(&request_str).unwrap();

    println!("Request: {:?}", request);

    let mut response = Megaresponse {
        int_codigo_retorno: 0,
        is_error: false,
        str_error_message: None,
        str_retorno: None,
        bin_retorno: None,
    };

    if request.sistema_id == 1 {
        response.int_codigo_retorno = 1;
        response.is_error = true;
        response.str_error_message = Some("Sistema não encontrado".to_string());
    } else {
        response.int_codigo_retorno = 0;
        response.is_error = false;
        response.str_retorno = Some("Sucesso".to_string());
    }

    retorno = response;

    Ok(retorno)
}

fn handle_response(mut socket: TcpStream, mensagen: String) -> Result<(), Box<dyn Error>> {
    let mut buf = [0; 1024];
    let mut response = String::new();

    socket.write_all(mensagen.as_bytes());
    socket.flush();

    Ok(())
   
}

fn cipher_vec(vec: Vec<u8>, key: u8) -> Vec<u8> {
    let mut vec_cript = vec.clone();
    for i in 0..vec.len() {
        vec_cript[i] ^= key;
    }
    vec_cript
}
use bytes::{BufMut, BytesMut};
use encoding::all::WINDOWS_1252;
use encoding::{EncoderTrap, Encoding};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
struct ProxyConfig {
    tcp_listener: String,
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
    bin_response: Option<Vec<u8>>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MegaresponseMinimal {
    int_codigo_retorno: i32,
    is_error: bool,
    str_error_message: Option<String>,
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
        eprintln!("{}", GLOBAL_CONFIG.tcp_listener);
        let tcp_listener = TcpListener::bind(GLOBAL_CONFIG.tcp_listener.clone()).await?;
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
    let mut rec_data: Vec<u8> = vec![]; // = vec![0; 256];

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

        //println!("Bytes lidos: {:?}, Total: {:?}", bytes_read, total_bytes);

        let mut rec_cript = cipher_vec(
            rec_buffer.clone(),
            GLOBAL_CONFIG.vch_key.clone().into_bytes()[0],
            bytes_read,
        );
        rec_data.append(&mut rec_cript);

        //eprint!("Buffer: {:?} ", rec_data);

        let end_of_message = rec_data.ends_with(b"<EOF><EOT>") || rec_data.ends_with(b"<EOF><EOM>");

        if end_of_message {
            let tipo_mensagem_imagem = rec_data.starts_with(b"101") || rec_data.starts_with(b"0101");
            let retorno = process_request(rec_data.clone()).await;
            match retorno {
                Ok(response) => {
                    //DO Json vem como UTF-8, para o terminal precisa ser Windows-1252
                    let response_bytes: Vec<u8>;
                    if tipo_mensagem_imagem {
                        let ret = response.bin_response;
                        if ret.is_none() {
                            eprintln!("Erro ao processar imagem");
                            response_bytes = prepare_encoding(response.str_retorno.to_owned().unwrap(), tipo_mensagem_imagem);
                            //break;
                        }else{
                            response_bytes = ret.unwrap();
                        }
                        //response_bytes = ret.unwrap();
                    } else {
                        response_bytes = prepare_encoding(response.str_retorno.to_owned().unwrap(), tipo_mensagem_imagem);
                    }
                    let len = response_bytes.len();
                    let response_cript = cipher_vec(
                        response_bytes,
                        GLOBAL_CONFIG.vch_key.clone().into_bytes()[0],
                        len,
                    );
                    let response_final = prepare_send_data(response_cript);
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

    eprintln!("Tipo Mensagem STR: {:?}", tokens[0]);
    let tipo_mensagem = tokens[0].parse::<i32>().unwrap_or(2); //Cair no mega em caso de erro
    eprintln!("Tipo Mensagem i32: {:?}", tipo_mensagem);
    let sistema_id: i32 = match tipo_mensagem {
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

    //eprintln!("Request REST enviado: {:?}", send_obj);
    let response: Megaresponse = reqwest::Client::new()
        .post(GLOBAL_CONFIG.server_url.as_str())
        .json(&send_obj)
        .send()
        .await?
        .json()
        .await?;

    eprintln!("Request REST recebido: {:?}", response);

    Ok(response)

    // if tipo_mensagem == 101 {
        
    //     let content = reqwest::Client::new()
    //             .post(GLOBAL_CONFIG.server_url.as_str())
    //             .json(&send_obj)
    //             .send()
    //             .await?
    //             .text()
    //             .await?;

    //     eprintln!("Request TEXT IMG recebido: {:?}", content);

    //     let check_response : MegaresponseMinimal = serde_json::from_str(content.as_str()).unwrap();
    //     if check_response.is_error{
    //         return Ok(Megaresponse{
    //             int_codigo_retorno: check_response.int_codigo_retorno,
    //             is_error: check_response.is_error,
    //             str_error_message: check_response.str_error_message,
    //             str_retorno: None,
    //             bin_response: None,
    //         });
    //     }

    //     let bin_ret = &content[content.find("binRetorno").unwrap()+13..content.len()-3];

    //     let response_final = Megaresponse {
    //         int_codigo_retorno: check_response.int_codigo_retorno,
    //         is_error: check_response.is_error,
    //         str_error_message : check_response.str_error_message,
    //         str_retorno: Some(bin_ret.to_string()),
    //         bin_response: check_response.bin_response
            
    //     };

    //     Ok(response_final)

    // }else{
    //     let response: Megaresponse = reqwest::Client::new()
    //             .post(GLOBAL_CONFIG.server_url.as_str())
    //             .json(&send_obj)
    //             .send()
    //             .await?
    //             .json()
    //             .await?;

    //     eprintln!("Request REST recebido: {:?}", response);

    //     Ok(response)
    // }

}

fn cipher_vec(vec_cipher: Vec<u8>, key: u8, size: usize) -> Vec<u8> {
    let mut vec_cript: Vec<u8> = vec![];
    for i in 0..size {
        vec_cript.push(vec_cipher[i] ^ key);
        //vec_cript.push(vec_cipher[i]);
    }
    vec_cript
}

//Adicionar 4 bytes com tamannho da mensagem e @ no final
fn prepare_send_data(vec: Vec<u8>) -> Vec<u8> {
    let mut buf = BytesMut::with_capacity(4 + vec.len());
    buf.put_u32_le(5 + vec.len() as u32);
    buf.put(&vec[..]);
    buf.put_u8(b'@');
    buf.to_vec()
}

fn prepare_encoding(str: String, bypass: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    if !bypass {
        WINDOWS_1252
            .encode_to(str.as_str(), EncoderTrap::Ignore, &mut bytes)
            .unwrap();
    } else {
        bytes = str.as_bytes().to_vec();
    }
    bytes
}

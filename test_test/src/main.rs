// These require the `serde` dependency.
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct SistemaVersao {
    #[serde(rename = "Ambiente_ID")]
    ambiente_id: i32,
    #[serde(rename = "Sistema_ID")]
    sistema_id: i32,
    #[serde(rename = "RequestAmbiente_ID")]
    request_ambiente_id: i32,
    #[serde(rename = "RequestSistema_ID")]
    request_sistema_id: i32,
    #[serde(rename = "vchVersao")]
    vch_versao: String,
    #[serde(rename = "vchIP")]
    vch_ip: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaseResponse {
    //#[serde(rename = "intCodigoRetorno")]
    int_codigo_retorno: i32,
    //#[serde(rename = "strMensagem")]
    str_mensagem: Option<String>,
    //#[serde(rename = "strErrorMessage")]
    str_error_message: Option<String>,
    //#[serde(rename = "strExcecao")]
    str_excecao: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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

fn main() {
    let config: ProxyConfig = read_config_from_file("./config.json").unwrap();

    println!("{:?}", config);

    let _result = async {
        send_version().await?;
        Ok::<(), reqwest::Error>(())
    };
}

async fn send_version() -> Result<(), reqwest::Error> {
    let send_obj = SistemaVersao {
        ambiente_id: 10,
        sistema_id: 18,
        request_ambiente_id: 17,
        request_sistema_id: 30,
        vch_versao: "Rust".into(),
        vch_ip: "127.0.0.1".into(),
    };
    println!("{:#?}", send_obj);
    let ret_obj: BaseResponse = reqwest::Client::new()
        .post("https://extapp.gwsvc.de/api/ConfigAPI/SetSistemaVersao/")
        .json(&send_obj)
        .send()
        .await?
        .json()
        .await?;

    println!("{:#?}", ret_obj);

    Ok(())
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

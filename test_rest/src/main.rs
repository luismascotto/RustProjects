//! This example illustrates the way to send and receive statically typed JSON.
//!
//! In contrast to the arbitrary JSON example, this brings up the full power of
//! Rust compile-time type system guaranties though it requires a little bit
//! more code.

// These require the `serde` dependency.
use serde::{Deserialize, Serialize};

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
struct BaseResponse {
    #[serde(rename = "intCodigoRetorno")]
    int_codigo_retorno: i32,
    #[serde(rename = "strMensagem")]
    str_mensagem: Option<String>,
    #[serde(rename = "strErrorMessage")]
    str_error_message: Option<String>,
    #[serde(rename = "strExcecao")]
    str_excecao: Option<String>,
}
// This is using the `tokio` runtime. You'll need the following dependency:
//
// `tokio = { version = "1", features = ["full"] }`
#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
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

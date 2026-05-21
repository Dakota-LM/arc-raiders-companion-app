use reqwest::{Certificate, Client};
use std::sync::LazyLock;

pub fn build_metaforge_http_client() -> Result<Client, String> {
    let roots = webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .map(|cert| {
            Certificate::from_der(cert).map_err(|e| format!("Failed to load root cert: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Client::builder()
        .tls_backend_rustls()
        .tls_certs_only(roots)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))
}

pub static HTTP_CLIENT: LazyLock<Client> =
    LazyLock::new(|| build_metaforge_http_client().expect("Failed to build HTTP client"));

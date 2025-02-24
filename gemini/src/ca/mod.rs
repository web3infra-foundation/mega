use serde_json::json;

pub mod client;
pub mod server;

async fn save_to_vault(key: String, value: String) {
    let key_f = format!("ca_{key}");
    let kv_data = json!({
        key_f.clone(): value,
    })
    .as_object()
    .unwrap()
    .clone();
    vault::vault::write_secret(key_f.as_str(), Some(kv_data.clone()))
        .await
        .unwrap();
}

async fn get_from_vault(key: String) -> Option<String> {
    let key_f = format!("ca_{key}");
    let secret = match vault::vault::read_secret(key_f.as_str()).await.unwrap() {
        Some(res) => res.data,
        None => return None,
    };

    match secret {
        Some(m) => {
            let s = m.get(key_f.as_str()).unwrap().as_str().unwrap().to_string();
            let s = s.trim_matches(char::is_control).to_string();
            Some(s)
        }
        None => None,
    }
}

async fn _delete_to_vault(key: String) {
    let key_f = format!("ca_{key}");
    vault::vault::write_secret(key_f.as_str(), None)
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {

    use quinn::rustls::pki_types::{pem::PemObject, CertificateSigningRequestDer};
    use rcgen::{
        generate_simple_self_signed, CertificateParams, CertificateSigningRequestParams,
        CertifiedKey, KeyPair,
    };

    #[tokio::test]
    async fn self_signed_cert() {
        let subject_alt_names = vec!["localhost".to_string()];

        let CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(subject_alt_names).unwrap();
        print!("root_cert:{}", cert.pem());

        let name = "localhost";
        let params = CertificateParams::new(vec![name.into()]).unwrap();

        let user_key_pair = KeyPair::generate().unwrap();
        let user_csr = params.serialize_request(&user_key_pair).unwrap();

        let csrd = CertificateSigningRequestDer::from_pem_slice(user_csr.pem().unwrap().as_bytes())
            .unwrap();
        let csrq = CertificateSigningRequestParams::from_der(&csrd).unwrap();
        let user_cert = csrq.signed_by(&cert, &key_pair).unwrap();

        // let c = CertificateSigningRequestDer::from_pem_slice(user_csr.pem().unwrap().as_bytes())
        //     .unwrap();
        // let user_cert = params.signed_by(&user_key_pair, &cert, &key_pair).unwrap();
        print!("user_cert:{}", user_cert.pem());
    }
}

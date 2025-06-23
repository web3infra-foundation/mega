use serde_json::json;
use vault::integration::{vault_core::VaultCoreInterface, VaultCore};

pub mod client;
pub mod server;

fn save_to_vault(vault: &VaultCore, key: String, value: String) {
    let key_f = format!("ca_{key}");
    let kv_data = json!({
        key_f.clone(): value,
    })
    .as_object()
    .unwrap()
    .clone();
    vault
        .write_secret(key_f.as_str(), Some(kv_data.clone()))
        .unwrap();
}

fn get_from_vault(vault: &VaultCore, key: String) -> Option<String> {
    let key_f = format!("ca_{key}");
    match vault.read_secret(key_f.as_str()).unwrap() {
        Some(res) => res
            .get(key_f.as_str())
            .map(|v| {
                v.as_str()
                    .map(|vv| String::from(vv.trim_matches(char::is_control)))
            })
            .flatten(),
        None => return None,
    }
}

fn _delete_to_vault(vault: &VaultCore, key: String) {
    let key_f = format!("ca_{key}");
    vault.delete_secret(key_f.as_str()).unwrap();
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

use quinn::rustls::pki_types::CertificateSigningRequestDer;
use quinn::rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use rcgen::{
    generate_simple_self_signed, CertificateParams, CertificateSigningRequestParams, CertifiedKey,
    KeyPair,
};

use anyhow::anyhow;
use anyhow::Result;
use vault::integration::VaultCore;

use crate::ca::save_to_vault;

use super::get_from_vault;

static ROOT_CERT: &str = "root_cert";
static ROOT_KEY: &str = "root_key";

static USER_KEY_PRE: &str = "user_";

pub fn get_root_cert_pem(vault: &VaultCore) -> String {
    match get_from_vault(vault, ROOT_CERT.to_string()) {
        Some(cert) => cert,
        None => init_self_signed_cert(vault).0,
    }
}

pub fn get_root_cert_der(vault: &VaultCore) -> CertificateDer<'static> {
    let cert = get_root_cert_pem(vault);
    let cert = CertificateDer::from_pem_slice(cert.as_bytes()).unwrap();
    cert
}

pub fn get_root_key_pem(vault: &VaultCore) -> String {
    match get_from_vault(vault, ROOT_KEY.to_string()) {
        Some(key) => key,
        None => init_self_signed_cert(vault).1,
    }
}

pub fn get_root_key_der(vault: &VaultCore) -> PrivateKeyDer<'static> {
    let key = get_root_key_pem(vault);
    let key = PrivateKeyDer::from_pem_slice(key.as_bytes()).unwrap();
    key
}

fn init_self_signed_cert(vault: &VaultCore) -> (String, String) {
    let subject_alt_names = vec!["localhost".to_string()];

    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names).unwrap();
    save_to_vault(vault, ROOT_CERT.to_string(), cert.pem());
    save_to_vault(vault, ROOT_KEY.to_string(), key_pair.serialize_pem());
    (cert.pem(), key_pair.serialize_pem())
}

pub fn get_certificate(vault: &VaultCore, name: String) -> Result<String> {
    if name == "ca" {
        return Ok(get_root_cert_pem(vault));
    }

    let cert_option = get_from_vault(vault, add_user_key_pre(name));
    match cert_option {
        Some(cert) => Ok(cert),
        None => Err(anyhow!("Username not found")),
    }
}

pub fn issue_certificate(vault: &VaultCore, name: String, csr: String) -> Result<String> {
    tracing::info!("sign_certificate, name:{name},csr:{csr}");
    let ca_key = KeyPair::from_pem(get_root_key_pem(vault).as_str()).unwrap();
    let params = CertificateParams::from_ca_cert_pem(get_root_cert_pem(vault).as_str()).unwrap();
    let ca_cert = params.self_signed(&ca_key).unwrap();

    let csrd = match CertificateSigningRequestDer::from_pem_slice(csr.as_bytes()) {
        Ok(csrd) => csrd,
        Err(e) => return Err(anyhow!(e.to_string())),
    };
    let csrq = CertificateSigningRequestParams::from_der(&csrd).unwrap();
    let user_cert = csrq.signed_by(&ca_cert, &ca_key).unwrap();

    save_to_vault(vault, add_user_key_pre(name), user_cert.pem());
    Ok(user_cert.pem())
}

fn _is_reserved_key(name: String) -> bool {
    if [ROOT_CERT.to_string(), ROOT_KEY.to_string()].contains(&name) {
        return true;
    }
    false
}

fn add_user_key_pre(name: String) -> String {
    format!("{}{}", USER_KEY_PRE, name)
}

pub fn get_cert_name_from_path(path: &str) -> Option<String> {
    let v: Vec<&str> = path.split('/').collect();
    v.get(3).map(|s| s.to_string())
}

pub fn get_hub_name_from_path(path: &str) -> Option<String> {
    let v: Vec<&str> = path.split('/').collect();
    v.get(4).map(|s| s.to_string())
}

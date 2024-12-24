use std::{
    env,
    fs::{File, OpenOptions},
    io::{Read, Write},
};

use base64::{Engine as _, engine::general_purpose};
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
use sha2::{Digest, Sha256};

fn main() {
    let url_str = env::var("URL").expect("env var URL not set");

    let mut changed_url = Vec::new();

    for url in url_str.split_whitespace() {
        if reslove(url) {
            eprintln!("Web page changed: {}", url);
            changed_url.push(url);
        }
    }

    if !changed_url.is_empty() {
        eprintln!("Sending email");
        send_email(&changed_url);
    } else {
        eprintln!("Web pages are not changed");
    }
    eprintln!("Done");
}

fn reslove(url: &str) -> bool {
    eprintln!("Start fetching: {}", url);
    let result = reqwest::blocking::get(url);
    if result.is_err() {
        eprintln!("Error fetching: {}", result.err().unwrap());
        std::process::exit(1);
    }
    let response = result.unwrap();
    if !response.status().is_success() {
        eprintln!("Error fetching: {}", response.status());
        std::process::exit(1);
    }
    let data = response.bytes().expect("Error reading response");
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    // hash bytes to base64
    compare_store(&url, hash)
}

fn compare_store<T: AsRef<[u8]>>(url: &str, hash: T) -> bool {
    // let tmpdir = TempDir::new("webwatcher").expect("Error creating temp dir");
    let tmpdir = env::temp_dir();
    let filename = general_purpose::STANDARD.encode(url);
    let hash_file = tmpdir.join(format!("web_watcher_{}", filename));
    let exists = hash_file.exists();
    if exists {
        let mut file = File::open(&hash_file).expect("Error opening hash file");
        // read as bytes
        let mut old_hash = Vec::new();
        file.read_to_end(&mut old_hash)
            .expect("Error reading hash file");
        if old_hash.eq(hash.as_ref()) {
            return false;
        }
    }
    eprintln!("Writing hash file: {}", hash_file.display());
    // overwrite or create hash file
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&hash_file)
        .expect(format!("Error creating hash file: {}", hash_file.to_string_lossy()).as_str());
    file.write_all(hash.as_ref())
        .expect("Error writing hash file");

    exists
}

fn send_email(url: &Vec<&str>) {
    let username = env::var("EMAIL_USERNAME").expect("env var EMAIL_USERNAME not set");
    let password = env::var("EMAIL_PASSWORD").expect("env var EMAIL_PASSWORD not set");
    let server = env::var("EMAIL_SERVER").expect("env var EMAIL_SERVER not set");
    let to = env::var("EMAIL_TO").expect("env var EMAIL_TO not set");
    let url_str = url.join("<br>");
    let email = Message::builder()
        .from(format!("web watcher <{}>", username).parse().unwrap())
        .to(to.parse().unwrap())
        .subject("网站更新提醒")
        .header(ContentType::TEXT_HTML)
        .body(format!("网站地址: {}", url_str))
        .unwrap();
    let creds = Credentials::new(username, password);

    let mailer;
    // check if port in server
    if server.contains(':') {
        let parts = server.rsplit_once(':').unwrap();
        mailer = SmtpTransport::relay(parts.0)
            .unwrap()
            .credentials(creds)
            .port(parts.1.parse().unwrap())
            .build();
    } else {
        mailer = SmtpTransport::relay(&server)
            .unwrap()
            .credentials(creds)
            .build();
    }

    match mailer.send(&email) {
        Ok(_) => eprintln!("Email sent"),
        Err(e) => eprintln!("Error sending email: {}", e),
    }
}

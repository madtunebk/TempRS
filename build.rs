// Build script to load .env variables at compile time
fn main() {
    // Load .env file if it exists
    if let Err(e) = dotenvy::dotenv() {
        println!("cargo:warning=Failed to load .env file: {}", e);
        println!("cargo:warning=Copy .env.example to .env and add your SoundCloud credentials");
        std::process::exit(1);
    }
    
    // Read credentials from environment and pass to rustc
    let client_id = std::env::var("SOUNDCLOUD_CLIENT_ID")
        .expect("SOUNDCLOUD_CLIENT_ID must be set in .env file");
    let client_secret = std::env::var("SOUNDCLOUD_CLIENT_SECRET")
        .expect("SOUNDCLOUD_CLIENT_SECRET must be set in .env file");
    
    println!("cargo:rustc-env=SOUNDCLOUD_CLIENT_ID={}", client_id);
    println!("cargo:rustc-env=SOUNDCLOUD_CLIENT_SECRET={}", client_secret);
    println!("cargo:rerun-if-changed=.env");
}

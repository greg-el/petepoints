// #[macro_use]
// extern crate dotenv_codegen;

// #[macro_use]
// extern crate lazy_static;

// mod discord;
mod web;

#[tokio::main]
async fn main() {
    //discord::start_watcher();

    let web = match web::start_watcher().await {
        Ok(r) => r,
        Err(error) => panic!("Oh no, {}", error),
    };
}

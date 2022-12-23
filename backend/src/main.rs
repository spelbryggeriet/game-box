use rocket::{
    fs::{relative, FileServer},
    launch,
};

#[launch]
async fn rocket() -> _ {
    rocket::build().mount("/", FileServer::from(relative!("../frontend/dist/")))
}

use crate::rocket::{Build, Rocket};

cached_static_response_handler! {
    259_200;
    "/web-app.manifest" => web_app_manifest => "web-app.manifest",
    "/browser-config.xml" => browser_config => "browser-config",
    "/favicon-monochrome.svg" => favicon_monochrome => "favicon-monochrome",
    "/favicon.ico" => favicon => "favicon",
    "/favicon-512.png" => favicon_512 => "favicon-512",
    "/favicon-192.png" => favicon_192 => "favicon-192",
    "/favicon-32.png" => favicon_32 => "favicon-32",
    "/favicon-16.png" => favicon_16 => "favicon-16",
    "/favicon-180-i.png" => favicon_180_i => "favicon-180-i",
    "/mstile-310.png" => mstile_310 => "mstile-310",
    "/mstile-150.png" => mstile_150 => "mstile-150",
    "/mstile-70.png" => mstile_70 => "mstile-70",
}

pub fn mounts(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/", routes![web_app_manifest])
        .mount("/", routes![browser_config])
        .mount("/", routes![favicon_monochrome])
        .mount("/", routes![favicon])
        .mount("/", routes![favicon_512, favicon_192, favicon_32, favicon_16])
        .mount("/", routes![favicon_180_i])
        .mount("/", routes![mstile_310, mstile_150, mstile_70])
}

mod bundles;
mod favicons;

use crate::rocket::Rocket;
use crate::rocket_include_static_resources::StaticResponse;

pub fn rocket_handler(rocket: Rocket) -> Rocket {
    let rocket = rocket.attach(StaticResponse::fairing(|resources| {
        static_resources_initialize!(
            resources,

            "css-bundle", "front-end/css/bundle.min.css",
            "js-bundle", "front-end/js/bundle.min.js",

            "font-roboto-mono", "front-end/css/font-roboto-mono.min.css",

            "RobotoMono-Bold", "front-end/fonts/RobotoMono-Bold.woff2",
            "RobotoMono-Light", "front-end/fonts/RobotoMono-Light.woff2",
            "RobotoMono-Medium", "front-end/fonts/RobotoMono-Medium.woff2",
            "RobotoMono-Regular", "front-end/fonts/RobotoMono-Regular.woff2",

            "fa-brands-400-eot", "front-end/fonts/fa-brands-400.eot",
            "fa-brands-400-svg", "front-end/fonts/fa-brands-400.svg",
            "fa-brands-400-ttf", "front-end/fonts/fa-brands-400.ttf",
            "fa-brands-400-woff", "front-end/fonts/fa-brands-400.woff",
            "fa-brands-400-woff2", "front-end/fonts/fa-brands-400.woff2",
            "fa-regular-400-eot", "front-end/fonts/fa-regular-400.eot",
            "fa-regular-400-svg", "front-end/fonts/fa-regular-400.svg",
            "fa-regular-400-ttf", "front-end/fonts/fa-regular-400.ttf",
            "fa-regular-400-woff", "front-end/fonts/fa-regular-400.woff",
            "fa-regular-400-woff2", "front-end/fonts/fa-regular-400.woff2",
            "fa-solid-900-eot", "front-end/fonts/fa-solid-900.eot",
            "fa-solid-900-svg", "front-end/fonts/fa-solid-900.svg",
            "fa-solid-900-ttf", "front-end/fonts/fa-solid-900.ttf",
            "fa-solid-900-woff", "front-end/fonts/fa-solid-900.woff",
            "fa-solid-900-woff2", "front-end/fonts/fa-solid-900.woff2",

            "preload", "front-end/images/preload.svg",

            "web-app.manifest", "front-end/web-app.manifest",
            "browser-config", "front-end/browser-config.xml",
            "favicon-monochrome", "front-end/favicon-monochrome.svg",
            "favicon", "front-end/favicon.ico",
            "favicon-512", "front-end/favicon-512.png",
            "favicon-310", "front-end/favicon-310.png",
            "favicon-192", "front-end/favicon-192.png",
            "favicon-150", "front-end/favicon-150.png",
            "favicon-70", "front-end/favicon-70.png",
            "favicon-32", "front-end/favicon-32.png",
            "favicon-16", "front-end/favicon-16.png",
            "favicon-180-i", "front-end/favicon-180-i.png",
        );
    }));

    let rocket = bundles::mounts(rocket);
    let rocket = favicons::mounts(rocket);

    rocket
}
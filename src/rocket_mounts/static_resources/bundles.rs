use rocket::{Build, Rocket};

cached_static_response_handler! {
    259_200;
    "/css/bundle.min.css" => css_bundle => "css-bundle",
    "/js/bundle.min.js" => js_bundle => "js-bundle",
    "/css/font-roboto-mono.min.css" => font_roboto_mono => "font-roboto-mono",
    "/fonts/RobotoMono-Bold.woff2" => roboto_mono_bold => "RobotoMono-Bold",
    "/fonts/RobotoMono-Light.woff2" => roboto_mono_light => "RobotoMono-Light",
    "/fonts/RobotoMono-Medium.woff2" => roboto_mono_medium => "RobotoMono-Medium",
    "/fonts/RobotoMono-Regular.woff2" => roboto_mono_regular => "RobotoMono-Regular",
    "/fonts/fa-solid-900.eot" => fa_solid_900_eot => "fa-solid-900-eot",
    "/fonts/fa-solid-900.svg" => fa_solid_900_svg => "fa-solid-900-svg",
    "/fonts/fa-solid-900.ttf" => fa_solid_900_ttf => "fa-solid-900-ttf",
    "/fonts/fa-solid-900.woff" => fa_solid_900_woff => "fa-solid-900-woff",
    "/fonts/fa-solid-900.woff2" => fa_solid_900_woff2 => "fa-solid-900-woff2",
    "/images/preload.svg" => preload => "preload",
}

pub fn mounts(rocket: Rocket<Build>) -> Rocket<Build> {
    rocket
        .mount("/", routes![css_bundle, js_bundle])
        .mount("/", routes![font_roboto_mono])
        .mount("/", routes![
            roboto_mono_bold,
            roboto_mono_light,
            roboto_mono_medium,
            roboto_mono_regular
        ])
        .mount("/", routes![
            fa_solid_900_eot,
            fa_solid_900_svg,
            fa_solid_900_ttf,
            fa_solid_900_woff,
            fa_solid_900_woff2
        ])
        .mount("/", routes![preload])
}

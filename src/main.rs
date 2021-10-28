use std::env;
use std::error::Error;

use clap::App;
use clap::Arg;
use clap::SubCommand;
use serde::Deserialize;
use serde::Serialize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    let token = env::var("LIFX_TOKEN")?;

    let lights = client
        .get("https://api.lifx.com/v1/lights/all")
        .bearer_auth(&token)
        .send()
        .await?
        .json::<Vec<Light>>()
        .await?;

    let matches = App::new("LiFX Controller")
        .version("1.0")
        .author("David Muir <hey@davidmuir.co>")
        .about("Allows controlling the Lifx bulbs in my home")
        .subcommand(
            SubCommand::with_name("set")
                .about("Updates state of one or more lights")
                .arg(Arg::with_name("on").long("on"))
                .arg(Arg::with_name("off").long("off"))
                .arg(
                    Arg::with_name("colour")
                        .long("colour")
                        .short("c")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("brightness")
                        .long("brightness")
                        .short("b")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("selector")
                        .long("selector")
                        .short("s")
                        .takes_value(true),
                ),
        )
        .subcommand(SubCommand::with_name("list").about("Lists all available lights"))
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("set") {
        let state = SetState {
            power: if matches.is_present("on") {
                Some(Power::On)
            } else if matches.is_present("off") {
                Some(Power::Off)
            } else {
                None
            },
            brightness: if let Some(b) = matches.value_of("brightness") {
                Some(b.parse::<f32>()?)
            } else {
                None
            },
            color: if let Some(c) = matches.value_of("colour") {
                Some(c.into())
            } else {
                None
            },
        };

        if let Some(selector) = matches.value_of("selector") {
            for l in lights.into_iter().filter(|l| {
                l.label
                    .to_ascii_lowercase()
                    .contains(&selector.to_ascii_lowercase())
                    || l.group
                        .name
                        .to_ascii_lowercase()
                        .contains(&selector.to_ascii_lowercase())
            }) {
                print!("Updating {} in {}", l.label, l.group.name);

                let mut url = "https://api.lifx.com/v1/lights/id:".to_owned();

                url.push_str(&l.id);

                url.push_str("/state");

                let resp = client
                    .put(url)
                    .bearer_auth(&token)
                    .json(&state)
                    .send()
                    .await?
                    .json::<SetStateResponse>()
                    .await?;

                if let Some(results) = resp.results {
                    for r in results {
                        print!(" - {}", r.status);
                    }
                } else {
                    println!("Something went wrong - {:#?}", resp);
                }

                println!()
            }
        } else {
            println!("Setting all lights");

            let url = "https://api.lifx.com/v1/lights/all";

            let resp = client
                .put(url)
                .bearer_auth(&token)
                .json(&state)
                .send()
                .await?
                .json::<SetStateResponse>()
                .await?;

            if let Some(results) = resp.results {
                for r in results {
                    println!(" - {}", r.status);
                }
            } else {
                println!("Something went wrong - {:#?}", resp);
            }

            println!()
        };
    } else if let Some(_matches) = matches.subcommand_matches("list") {
        for light in lights.into_iter() {
            println!(
                "{} - {} - power:{:?}, brightness:{}, temperature:{}k",
                light.label, light.group.name, light.power, light.brightness, light.color.kelvin
            );
        }
    }

    Ok(())
}

#[derive(Serialize, Debug)]
struct SetState {
    power: Option<Power>,
    brightness: Option<f32>,
    color: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SetStateResponse {
    results: Option<Vec<SetStateResult>>,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SetStateResult {
    id: String,
    label: String,
    status: String,
}

#[derive(Deserialize, Debug)]
struct Light {
    id: String,
    uuid: String,
    label: String,
    connected: bool,
    power: Power,
    color: Colour,
    brightness: f32,
    group: Group,
    location: Group,
    product: Product,
    last_seen: String,
    seconds_since_seen: u32,
}

#[derive(Deserialize, Debug)]
struct Product {
    name: String,
    identifier: String,
    company: String,
    vendor_id: u8,
    product_id: u32,
}

#[derive(Deserialize, Debug)]
struct Group {
    id: String,
    name: String,
}

#[derive(Deserialize, Serialize, Debug)]
enum Power {
    #[serde(rename = "on")]
    On,
    #[serde(rename = "off")]
    Off,
}

#[derive(Deserialize, Debug)]
struct Colour {
    hue: u32,
    saturation: f32,
    kelvin: u32,
}

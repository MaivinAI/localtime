use cdr::{CdrLe, Infinite};
use chrono::{Datelike as _, Offset as _, Timelike as _};
use clap::Parser;
use edgefirst_schemas::{builtin_interfaces, edgefirst_msgs, std_msgs};
use serde_json::json;
use std::{error::Error, time::Duration};
use zenoh::{
    bytes::{Encoding, ZBytes},
    config::{Config, WhatAmI},
};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// topic name
    #[arg(long, default_value = "rt/localtime")]
    topic: String,

    /// zenoh connection mode
    #[arg(long, env, default_value = "peer")]
    mode: WhatAmI,

    /// connect to zenoh endpoints
    #[arg(long, env)]
    connect: Vec<String>,

    /// listen to zenoh endpoints
    #[arg(long, env)]
    listen: Vec<String>,

    /// disable zenoh multicast scouting
    #[arg(long, env)]
    no_multicast_scouting: bool,
}

impl From<Args> for Config {
    fn from(args: Args) -> Self {
        let mut config = Config::default();

        config
            .insert_json5("mode", &json!(args.mode).to_string())
            .unwrap();

        if !args.connect.is_empty() {
            config
                .insert_json5("connect/endpoints", &json!(args.connect).to_string())
                .unwrap();
        }

        if !args.listen.is_empty() {
            config
                .insert_json5("listen/endpoints", &json!(args.listen).to_string())
                .unwrap();
        }

        if args.no_multicast_scouting {
            config
                .insert_json5("scouting/multicast/enabled", &json!(false).to_string())
                .unwrap();
        }

        config
            .insert_json5("scouting/multicast/interface", &json!("lo").to_string())
            .unwrap();

        config
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    env_logger::init();

    let session = zenoh::open(args.clone()).await.unwrap();
    let publisher = session.declare_publisher(args.topic).await.unwrap();

    loop {
        let now = chrono::Local::now();
        let localtime = now.time();
        let timezone = now.offset().fix().local_minus_utc() / 60;

        let msg = edgefirst_msgs::LocalTime {
            header: std_msgs::Header {
                stamp: timestamp()?,
                frame_id: "".to_string(),
            },
            date: edgefirst_msgs::Date {
                year: now.year() as u16,
                month: now.month() as u8,
                day: now.day() as u8,
            },
            time: builtin_interfaces::Time {
                sec: localtime.num_seconds_from_midnight() as i32,
                nanosec: localtime.nanosecond(),
            },
            timezone: timezone as i16,
        };

        log::trace!("LocalTime: {:?}", msg);

        let msg = ZBytes::from(cdr::serialize::<_, _, CdrLe>(&msg, Infinite)?);
        let enc = Encoding::APPLICATION_CDR.with_schema("edgefirst_msgs/msg/LocalTime");
        publisher.put(msg).encoding(enc).await.unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn timestamp() -> Result<builtin_interfaces::Time, std::io::Error> {
    let mut tp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let err = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC_RAW, &mut tp) };
    if err != 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(builtin_interfaces::Time {
        sec: tp.tv_sec as i32,
        nanosec: tp.tv_nsec as u32,
    })
}

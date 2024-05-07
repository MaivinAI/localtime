use cdr::{CdrLe, Infinite};
use chrono::{Datelike as _, Offset as _, Timelike as _};
use clap::Parser;
use edgefirst_schemas::{builtin_interfaces, edgefirst_msgs, std_msgs};
use std::{
    error::Error,
    str::FromStr,
    time::{Duration, SystemTime},
};
use unix_ts::Timestamp;
use zenoh::{config::Config, prelude::r#async::*};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// zenoh connection mode
    #[arg(long, default_value = "client")]
    mode: String,

    /// connect to endpoint
    #[arg(short, long, default_value = "tcp/127.0.0.1:7447")]
    endpoint: Vec<String>,

    /// topic name
    #[arg(long, default_value = "rt/localtime")]
    topic: String,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    env_logger::init();

    let mut config = Config::default();
    let mode = WhatAmI::from_str(&args.mode)?;
    config.set_mode(Some(mode)).unwrap();
    config.connect.endpoints = args.endpoint.iter().map(|v| v.parse().unwrap()).collect();
    let _ = config.scouting.multicast.set_enabled(Some(false));
    let session = zenoh::open(config).res_async().await.unwrap().into_arc();
    log::info!(
        "Opened Zenoh session [mode: {} endpoint: {:?}]",
        args.mode,
        args.endpoint
    );

    let publisher = session.declare_publisher(args.topic).res().await.unwrap();

    loop {
        let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let ts = Timestamp::from_nanos(ts.as_nanos() as i128);
        let now = chrono::Local::now();
        let localtime = now.time();
        let timezone = now.offset().fix().local_minus_utc() / 60;

        let msg = edgefirst_msgs::LocalTime {
            header: std_msgs::Header {
                stamp: builtin_interfaces::Time {
                    sec: ts.seconds() as i32,
                    nanosec: ts.subsec(9),
                },
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

        let encoding = Encoding::WithSuffix(
            KnownEncoding::AppOctetStream,
            "edgefirst_msgs/msg/LocalTime".into(),
        );
        let encoded = cdr::serialize::<_, _, CdrLe>(&msg, Infinite)?;
        let encoded = Value::from(encoded).encoding(encoding);
        publisher.put(encoded).res().await.unwrap();
        async_std::task::sleep(Duration::from_secs(1)).await;
    }
}

use chrono::{Datelike as _, Offset as _, Timelike as _};
use r2r::{
    builtin_interfaces::msg::Time,
    edgefirst_msgs::msg::{Date, LocalTime},
    std_msgs::msg::Header,
    QosProfile,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let ctx = r2r::Context::create()?;
    let mut node = r2r::Node::create(ctx, "localtime", "")?;
    let publisher = node.create_publisher::<LocalTime>("/localtime", QosProfile::default())?;

    loop {
        let mut clock = r2r::Clock::create(r2r::ClockType::SteadyTime)?;
        let rostime = clock.get_now()?;
        let now = chrono::Local::now();
        let localtime = now.time();
        let timezone = now.offset().fix().local_minus_utc() / 60;

        let msg = LocalTime {
            header: Header {
                stamp: Time {
                    sec: rostime.as_secs() as i32,
                    nanosec: rostime.subsec_nanos(),
                },
                frame_id: "".to_string(),
            },
            date: Date {
                year: now.year() as u16,
                month: now.month() as u8,
                day: now.day() as u8,
            },
            time: Time {
                sec: localtime.num_seconds_from_midnight() as i32,
                nanosec: localtime.nanosecond(),
            },
            timezone: timezone as i16,
        };

        log::trace!("LocalTime: {:?}", msg);
        publisher.publish(&msg).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

//! edenfsctl uptime

use std::{fmt, time::Duration};

use async_trait::async_trait;
use structopt::StructOpt;

use edenfs_client::{DaemonHealthy, EdenFsInstance};
use edenfs_error::Result;

use crate::ExitCode;

#[derive(StructOpt, Debug)]
#[structopt(about = "Determine uptime of running edenfs daemon")]
pub struct UptimeCmd {}

struct HumanTime {
    days: u64,
    hours: u64,
    minutes: u64,
    seconds: u64,
}

impl From<Duration> for HumanTime {
    fn from(duration: Duration) -> HumanTime {
        let seconds_in_minutes = 60;
        let seconds_in_hours = 60 * seconds_in_minutes;
        let seconds_in_days = 24 * seconds_in_hours;

        let seconds = duration.as_secs();

        let days = seconds / seconds_in_days;
        let seconds = seconds % seconds_in_days;

        let hours = seconds / seconds_in_hours;
        let seconds = seconds % seconds_in_hours;

        let minutes = seconds / seconds_in_minutes;
        let seconds = seconds % seconds_in_minutes;

        HumanTime {
            days,
            hours,
            minutes,
            seconds,
        }
    }
}

impl fmt::Display for HumanTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            if self.days > 0 {
                write!(
                    f,
                    "{} day{}, ",
                    self.days,
                    if self.days == 1 { "" } else { "s" }
                )?;
            }
            write!(f, "{}:{:02}:{:02}", self.hours, self.minutes, self.seconds)
        } else {
            write!(
                f,
                "{}d:{:02}h:{:02}m:{:02}s",
                self.days, self.hours, self.minutes, self.seconds
            )
        }
    }
}

#[async_trait]
impl crate::Subcommand for UptimeCmd {
    async fn run(&self, instance: EdenFsInstance) -> Result<ExitCode> {
        let health = instance.get_health(None).await;

        match health {
            Ok(health) => {
                if health.is_healthy() {
                    let uptime = Duration::from_secs_f32(health.uptime.unwrap_or(0.0));
                    println!(
                        "edenfs uptime (pid: {}): {:#}",
                        health.pid,
                        HumanTime::from(uptime)
                    );
                } else {
                    println!("edenfs (pid: {}) is not healthy", health.pid);
                }
                Ok(0)
            }
            Err(cause) => {
                println!("edenfs not healthy: {}", cause);
                Ok(1)
            }
        }
    }
}

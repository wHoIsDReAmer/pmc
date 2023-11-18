pub mod fork;
pub mod pid;

use crate::helpers::{self, ColoredString};
use crate::process::Runner;
use crate::service;

use chrono::{DateTime, Utc};
use colored::Colorize;
use fork::{daemon, Fork};
use global_placeholders::global;
use macros_rs::{crashln, string, ternary, then};
use psutil::process::{MemoryInfo, Process};
use serde::Serialize;
use serde_json::json;
use std::{process, thread::sleep, time::Duration};

use tabled::{
    settings::{
        object::Columns,
        style::{BorderColor, Style},
        themes::Colorization,
        Color, Rotate,
    },
    Table, Tabled,
};

extern "C" fn handle_termination_signal(_: libc::c_int) {
    pid::remove();
    unsafe { libc::_exit(0) }
}

fn restart_process(runner: Runner) {
    let items = runner.list().iter().filter_map(|(id, item)| Some((id.trim().parse::<usize>().ok()?, item)));
    for (id, item) in items {
        then!(!item.running || pid::running(item.pid as i32), continue);
        let name = &Some(item.name.clone());
        let mut runner_instance = Runner::new();
        runner_instance.restart(id, name);
    }
}

pub fn health(format: &String) {
    let runner = Runner::new();
    let mut pid: Option<i32> = None;
    let mut cpu_percent: Option<f32> = None;
    let mut uptime: Option<DateTime<Utc>> = None;
    let mut memory_usage: Option<MemoryInfo> = None;

    #[derive(Clone, Debug, Tabled)]
    struct Info {
        #[tabled(rename = "pid file")]
        pid_file: String,
        #[tabled(rename = "fork path")]
        path: String,
        #[tabled(rename = "cpu percent")]
        cpu_percent: String,
        #[tabled(rename = "memory usage")]
        memory_usage: String,
        #[tabled(rename = "process count")]
        process_count: usize,
        uptime: String,
        pid: String,
        status: ColoredString,
    }

    impl Serialize for Info {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let trimmed_json = json!({
             "pid_file": &self.pid_file.trim(),
             "path": &self.path.trim(),
             "cpu": &self.cpu_percent.trim(),
             "mem": &self.memory_usage.trim(),
             "process_count": &self.process_count.to_string(),
             "uptime": &self.uptime.trim(),
             "pid": &self.pid.trim(),
             "status": &self.status.0.trim(),
            });

            trimmed_json.serialize(serializer)
        }
    }

    if pid::exists() {
        if let Ok(process_id) = pid::read() {
            if let Ok(mut process) = Process::new(process_id as u32) {
                pid = Some(process_id);
                uptime = Some(pid::uptime().unwrap());
                memory_usage = process.memory_info().ok();
                cpu_percent = process.cpu_percent().ok();
            }
        }
    }

    let cpu_percent = match cpu_percent {
        Some(percent) => format!("{:.2}%", percent),
        None => string!("0%"),
    };

    let memory_usage = match memory_usage {
        Some(usage) => helpers::format_memory(usage.rss()),
        None => string!("0b"),
    };

    let uptime = match uptime {
        Some(uptime) => helpers::format_duration(uptime),
        None => string!("none"),
    };

    let pid = match pid {
        Some(pid) => string!(pid),
        None => string!("n/a"),
    };

    let data = vec![Info {
        pid: pid,
        cpu_percent,
        memory_usage,
        uptime: uptime,
        path: global!("pmc.base"),
        pid_file: format!("{}  ", global!("pmc.pid")),
        process_count: runner.list().keys().len(),
        status: ColoredString(ternary!(pid::exists(), "online".green().bold(), "stopped".red().bold())),
    }];

    let table = Table::new(data.clone())
        .with(Rotate::Left)
        .with(Style::rounded().remove_horizontals())
        .with(Colorization::exact([Color::FG_CYAN], Columns::first()))
        .with(BorderColor::filled(Color::FG_BRIGHT_BLACK))
        .to_string();

    if let Ok(json) = serde_json::to_string(&data[0]) {
        match format.as_str() {
            "raw" => println!("{:?}", data[0]),
            "json" => println!("{json}"),
            "default" => {
                println!("{}\n{table}\n", format!("PMC daemon information").on_bright_white().black());
                println!(" {}", format!("Use `pmc daemon restart` to restart the daemon").white());
                println!(" {}", format!("Use `pmc daemon reset` to clean process id values").white());
            }
            _ => {}
        };
    };
}

pub fn stop() {
    if pid::exists() {
        println!("{} Stopping PMC daemon", *helpers::SUCCESS);

        match pid::read() {
            Ok(pid) => {
                service::stop(pid as i64);
                pid::remove();
                println!("{} PMC daemon stopped", *helpers::SUCCESS);
            }
            Err(err) => crashln!("{} Failed to read PID file: {}", *helpers::FAIL, err),
        }
    } else {
        crashln!("{} The daemon is not running", *helpers::FAIL)
    }
}

pub fn start() {
    pid::name("PMC Restart Handler Daemon");
    println!("{} Spawning PMC daemon with pmc_home={}", *helpers::SUCCESS, global!("pmc.base"));

    if pid::exists() {
        match pid::read() {
            Ok(pid) => then!(!pid::running(pid), pid::remove()),
            Err(_) => crashln!("{} The daemon is already running", *helpers::FAIL),
        }
    }

    println!("{} PMC Successfully daemonized", *helpers::SUCCESS);
    match daemon(false, false) {
        Ok(Fork::Parent(_)) => {}
        Ok(Fork::Child) => {
            unsafe { libc::signal(libc::SIGTERM, handle_termination_signal as usize) };
            pid::write(process::id());

            loop {
                let runner = Runner::new();
                then!(!runner.list().is_empty(), restart_process(runner));
                sleep(Duration::from_secs(1));
            }
        }
        Err(err) => {
            crashln!("{} Daemon creation failed with code {err}", *helpers::FAIL)
        }
    }
}

pub fn restart() {
    if pid::exists() {
        stop();
    }
    start();
}

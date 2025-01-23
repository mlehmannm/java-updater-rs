mod args;
#[cfg(feature = "azul")]
mod azul;
mod checksum;
mod colors;
mod config;
#[cfg(feature = "eclipse")]
mod eclipse;
mod meta;
#[cfg(feature = "notify")]
mod notify;
mod package;
mod util;
mod vars;
mod vendor;
mod version;

#[cfg(not(any(feature = "azul", feature = "eclipse")))]
compile_error!("At least one vendor must be set.");

use crate::args::Args;
use crate::colors::*;
use crate::config::*;
use crate::util::*;
use crate::version::Version;
use clap::Parser;
use std::path::{self, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use threadpool::ThreadPool;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};
use tracing::{level_filters::*, *};
use tracing_subscriber::EnvFilter;
use vendor::Vendor;

// Exit code used in case there were no errors.
#[doc(hidden)]
const EXIT_OK: i32 = 0;

// Exit code used in case of errors.
#[doc(hidden)]
const EXIT_NOK: i32 = 1;

/// Main entry point for the application.
fn main() {
    // enable ansi support to use colorised/styled output
    #[cfg(windows)]
    let _ = nu_ansi_term::enable_ansi_support();

    // delegate
    if let Err(err) = internal_main() {
        eprintln!("Failed! err = {err:#?}");
        std::process::exit(EXIT_NOK);
    } else {
        std::process::exit(EXIT_OK);
    }
}

// Internal main entry point for the application.
#[doc(hidden)]
fn internal_main() -> anyhow::Result<()> {
    // remember start date/time
    let start = Instant::now();

    // parse arguments
    let args = Args::parse();

    // print some information
    if !args.quiet || args.version {
        print_info();
    }

    // stop here in case only the version was requested
    if args.version {
        return Ok(());
    }

    // init tracing
    init_tracing(&args);

    // print parsed arguments
    trace!("arguments: {args:#?}");

    // load config
    let config_path = args.config.clone().unwrap_or(CONFIG_FILENAME.into());
    let config_path = PathBuf::from(config_path);
    let config_path = path::absolute(&config_path).unwrap_or(config_path);
    println!("Using configuration from {}.", PATH_COLOR.paint(config_path.to_string_lossy()));
    let config = Config::load_from_file(&config_path)?;
    debug!(?config);

    // derive base directory from config file.
    let Some(basedir) = config_path.parent() else {
        let message = "Failed to determine base directory!";
        println!("{}", ATTENTION_COLOR.paint(message));
        return Ok(());
    };
    debug!(basedir = %basedir.display());

    // start processing installations
    let thread_pool = ThreadPool::new(num_threads(args.threads));
    let args = Arc::new(args);
    let num_installations = config.installations.len();
    let processed = Arc::new(AtomicUsize::new(0));
    for installation in config.installations {
        let basedir = basedir.to_path_buf();
        let args = args.clone();
        let processed = processed.clone();
        thread_pool.execute(move || {
            setup(&basedir, &args, installation);

            // update window title
            let i = processed.fetch_add(1, Ordering::Relaxed);
            let window_title = format!("{i}/{num_installations} installs");
            set_window_title(&window_title);
        });
    }
    thread_pool.join();

    // print some statistics
    let elapsed = start.elapsed();
    println!("Total time: {}", format_elapsed(elapsed));
    let now = OffsetDateTime::now_local()?;
    println!("Finished at: {}", format_now(now));

    Ok(())
}

// Factor to compute the threads.
const THREADS_FACTOR: usize = 2;

// Computes the number of threads to use.
#[doc(hidden)]
fn num_threads(threads: Option<usize>) -> usize {
    // at least one thread
    let min_threads = 1;
    // at most max_threads
    let max_threads = std::thread::available_parallelism().map_or(min_threads, std::num::NonZeroUsize::get);
    // most of the tasks are I/O bound, so we can use more threads than available parallelism
    let max_threads = max_threads * THREADS_FACTOR;
    threads.unwrap_or(max_threads).clamp(min_threads, max_threads)
}

// TODO short doc
#[doc(hidden)]
fn format_elapsed(elapsed: Duration) -> String {
    // null out everything below seconds
    let elapsed = Duration::from_secs(elapsed.as_secs());

    // format the remaining duration
    humantime::format_duration(elapsed).to_string()
}

// TODO short doc
#[doc(hidden)]
fn format_now(now: OffsetDateTime) -> String {
    // define format
    const FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day] [hour]:[minute]:[second][offset_hour sign:mandatory][offset_minute]");

    // local offset or UTC
    let offset = UtcOffset::current_local_offset();
    let offset = offset.unwrap_or(UtcOffset::UTC);
    trace!(?offset);

    // format
    let now = now.to_offset(offset);
    now.format(&FORMAT).unwrap_or(now.to_string())
}

// Prints some information (version, path of executable, etc.).
#[doc(hidden)]
fn print_info() {
    let version = Version::default();
    if let Ok(exe) = std::env::current_exe() {
        let exe = PATH_COLOR.paint(exe.to_string_lossy());
        println!("{version} [{exe}]");
    } else {
        println!("{version}");
    }
}

// Initialises the tracing framework based on given command line arguments.
#[doc(hidden)]
fn init_tracing(args: &Args) {
    // `init` does call `set_logger`, so this is all we need to do.
    // We are falling back to printing all logs at info-level or above
    // if the RUST_LOG environment variable has not been set.
    // let env_logger_env = env_logger::Env::default().default_filter_or("info");
    // env_logger::Builder::from_env(env_logger_env).init();

    let level_filter = match args.verbose {
        0 => LevelFilter::ERROR.into(),
        1 => LevelFilter::WARN.into(),
        2 => LevelFilter::INFO.into(),
        3 => LevelFilter::DEBUG.into(),
        _ => LevelFilter::TRACE.into(),
    };
    let env_filter = EnvFilter::from_default_env().add_directive(level_filter);
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
}

// Set up installation.
fn setup(basedir: &Path, args: &Args, config: InstallationConfig) {
    let path = basedir.join(config.expand_directory());
    let path = path::absolute(&path).unwrap_or(path);
    let path = PATH_COLOR.paint(path.to_string_lossy());

    if !config.enabled {
        let not = ATTENTION_COLOR.paint("NOT");
        println!("{not} processing installation at {path} -> disabled");
        return;
    }

    let vendor = config.vendor.as_str();
    let Ok(vendor) = Vendor::try_from(vendor) else {
        let not = ATTENTION_COLOR.paint("NOT");
        println!("{not} processing installation at {path} -> unsupported vendor '{vendor}'");
        return;
    };
    trace!(?vendor);

    match vendor {
        #[cfg(feature = "azul")]
        Vendor::Azul => azul::setup(basedir, args, config),
        #[cfg(feature = "eclipse")]
        Vendor::Eclipse => eclipse::setup(basedir, args, config),
    };
}

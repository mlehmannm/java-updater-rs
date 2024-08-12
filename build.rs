//! Build script.

#[cfg(windows)]
use std::env;
use std::error::Error;
#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::process::{Command, Stdio};
use vergen_git2::{BuildBuilder, Emitter, Git2Builder, RustcBuilder};
#[cfg(windows)]
use windows_registry::LOCAL_MACHINE;
#[cfg(windows)]
use winres::WindowsResource;

#[cfg(windows)]
const RES_TARGET: &str = "res";
#[cfg(windows)]
const EXE_ICO_SOURCE: &str = "res/svg/exe.svg";
#[cfg(windows)]
const EXE_ICO_TARGET: &str = "res/exe.ico";
#[cfg(windows)]
const EXE_MANIFEST: &str = "res/exe.manifest";
#[cfg(windows)]
const CONVERT_EXE: &str = "convert.exe";
#[cfg(windows)]
const MAGICK_EXE: &str = "magick.exe";

// Main entry point for the build script.
fn main() -> Result<(), Box<dyn Error>> {
    // fetch some version information
    Emitter::default()
        .add_instructions(&BuildBuilder::all_build()?)?
        .add_instructions(&Git2Builder::all_git()?)?
        .add_instructions(&RustcBuilder::all_rustc()?)?
        .emit_and_set()?;

    // customise the executable (windows only)
    #[cfg(windows)]
    include_windows_resources()?;

    Ok(())
}

// Include some resources for windows builds to customise the executable.
#[cfg(windows)]
fn include_windows_resources() -> Result<(), Box<dyn Error>> {
    svg_to_ico(EXE_ICO_SOURCE, EXE_ICO_TARGET)?;
    let git_describe = std::env::var("VERGEN_GIT_DESCRIBE")?;
    println!("cargo:warning=build.rs: git_describe={git_describe}");
    let file_description = format!("Java Updater (git/{git_describe})");
    WindowsResource::new() //
        .set_icon(EXE_ICO_TARGET) //
        .set_manifest_file(EXE_MANIFEST) //
        .set_language(0x0407) // German (Germany)
        .set("FileDescription", &file_description)
        .compile()?;

    // don't rebuild when nothing changed
    println!("cargo:rerun-if-changed={RES_TARGET}");
    println!("cargo:rerun-if-changed={EXE_ICO_SOURCE}");
    println!("cargo:rerun-if-changed={EXE_ICO_TARGET}");
    println!("cargo:rerun-if-changed={EXE_MANIFEST}");

    Ok(())
}

// creates an icon from an svg
#[cfg(windows)]
fn svg_to_ico(input: &str, output: &str) -> Result<(), Box<dyn Error>> {
    if !is_outdated(input, output)? {
        return Ok(());
    }

    find_convert()? //
        .stdin(Stdio::null()) // disconnect from process
        .stderr(Stdio::null()) // disconnect from process
        .stdout(Stdio::null()) // disconnect from process
        .args(["-density", "256x256"]) //
        .args(["-background", "transparent"]) //
        .arg(input) //
        .args(["-define", "icon:auto-resize=256,64,48,40,32,24,20,16"]) //
        .args(["-compress", "none"])
        .arg(output) //
        .status()
        .inspect(|rc| println!("cargo:warning={CONVERT_EXE} returned {rc:?}!"))?;

    Ok(())
}

// checks whether the given output file is outdated in relation to the given input file
#[cfg(windows)]
fn is_outdated<P1, P2>(input: P1, output: P2) -> Result<bool, Box<dyn Error>>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    use std::time::SystemTime;

    let out_meta = fs::metadata(output);
    if let Ok(meta) = out_meta {
        let now = SystemTime::now();
        let output_mtime = meta.modified()?;

        // reduce resolution to prevent git checkout instablitities
        let output_mtime = now.duration_since(output_mtime)?.as_secs() / 2;

        let input_meta = fs::metadata(input)?;
        let input_mtime = input_meta.modified()?;

        // reduce resolution to prevent git checkout instablitities
        let input_mtime = now.duration_since(input_mtime)?.as_secs() / 2;

        // if input file is more recent than output file, we are outdated
        Ok(input_mtime > output_mtime)
    } else {
        // output file not found, we are outdated
        Ok(true)
    }
}

#[cfg(windows)]
fn find_convert() -> Result<Command, Box<dyn Error>> {
    if let Ok(convert) = find_convert_reg() {
        return Ok(convert);
    };

    find_convert_env()
}

#[cfg(windows)]
fn find_convert_reg() -> Result<Command, Box<dyn Error>> {
    let bin_path = LOCAL_MACHINE.open(r"Software\ImageMagick\Current")?.get_string("BinPath")?;

    // check for convert
    let convert = PathBuf::from(&bin_path).join(CONVERT_EXE);
    if fs::metadata(&convert).is_ok() {
        return Ok(Command::new(convert));
    }

    // check for magick
    let magick = PathBuf::from(&bin_path).join(MAGICK_EXE);
    let _ = fs::metadata(&magick)?;

    let mut command = Command::new(magick);
    command.arg("convert");

    Ok(command)
}

#[cfg(windows)]
fn find_convert_env() -> Result<Command, Box<dyn Error>> {
    let home = env::var("MAGICK_HOME")?;

    // check for convert
    let convert = PathBuf::from(&home).join(CONVERT_EXE);
    if fs::metadata(&convert).is_ok() {
        return Ok(Command::new(convert));
    }

    // check for magick
    let magick = PathBuf::from(&home).join(MAGICK_EXE);
    let _ = fs::metadata(&magick)?;

    let mut command = Command::new(magick);
    command.arg("convert");

    Ok(command)
}

use point_add_route_refactor::{
    compare_ops_artifacts, parse_route_preset, render_patch_plan_report, render_route_dump,
};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn default_route_preset_path() -> PathBuf {
    PathBuf::from("../../src/point_add/route_preset.rs")
}

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next();
    if matches!(first.as_deref(), Some("--formal-check")) {
        std::process::exit(run_formal_check());
    }
    if matches!(first.as_deref(), Some("--compare-ops")) {
        let Some(reference) = args.next() else {
            eprintln!("usage: point_add_route_refactor --compare-ops <reference> <candidate>");
            std::process::exit(2);
        };
        let Some(candidate) = args.next() else {
            eprintln!("usage: point_add_route_refactor --compare-ops <reference> <candidate>");
            std::process::exit(2);
        };
        if args.next().is_some() {
            eprintln!("usage: point_add_route_refactor --compare-ops <reference> <candidate>");
            std::process::exit(2);
        }
        match compare_ops_artifacts(reference, candidate) {
            Ok(comparison) => print!("{}", comparison.render()),
            Err(err) => {
                eprintln!("failed to compare ops artifacts: {err}");
                std::process::exit(4);
            }
        }
        return;
    }
    let (command, path) = match first.as_deref() {
        Some("--dump") | None => (
            "dump",
            args.next()
                .map(PathBuf::from)
                .unwrap_or_else(default_route_preset_path),
        ),
        Some("--report") => (
            "report",
            args.next()
                .map(PathBuf::from)
                .unwrap_or_else(default_route_preset_path),
        ),
        Some(path) => ("dump", PathBuf::from(path)),
    };

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("failed to read {}: {err}", path.display());
            std::process::exit(2);
        }
    };
    let preset = match parse_route_preset(&source) {
        Ok(preset) => preset,
        Err(err) => {
            eprintln!("failed to parse route preset: {err}");
            std::process::exit(3);
        }
    };

    match command {
        "dump" => print!("{}", render_route_dump(&preset, &BTreeMap::new())),
        "report" => print!("{}", render_patch_plan_report(&preset)),
        _ => unreachable!(),
    }
}

fn run_formal_check() -> i32 {
    let formal_dir = match find_formal_dir() {
        Some(path) => path,
        None => {
            eprintln!("failed to locate dev/formal from current directory");
            return 2;
        }
    };

    let mut z3 = Command::new("python");
    z3.arg(formal_dir.join("check_solinas_cuccaro_z3.py"));

    let mut lean = lean_command();
    lean.arg(formal_dir.join("SolinasCuccaroFacts.lean"));

    let checks = [
        ("z3", run_command("z3", &mut z3)),
        (
            "tla_core",
            run_tla_config(
                &formal_dir,
                "PointAddSolinasCuccaroAdder.cfg",
                "tlc-states-core",
            ),
        ),
        (
            "tla_q1175",
            run_tla_config(
                &formal_dir,
                "PointAddSolinasCuccaroAdderQ1175.cfg",
                "tlc-states-q1175",
            ),
        ),
        ("lean", run_command("lean", &mut lean)),
    ];

    if checks.iter().all(|(_, ok)| *ok) {
        println!("formal_check=pass");
        0
    } else {
        eprintln!("formal_check=fail");
        for (name, ok) in checks {
            if !ok {
                eprintln!("failed={name}");
            }
        }
        1
    }
}

fn run_tla_config(formal_dir: &std::path::Path, config_name: &str, state_dir: &str) -> bool {
    let mut tla = Command::new("java");
    tla.arg("-cp")
        .arg(formal_dir.join("tools").join("tla2tools.jar"))
        .arg("tlc2.TLC")
        .arg("-metadir")
        .arg(formal_dir.join(state_dir))
        .arg("-config")
        .arg(formal_dir.join(config_name))
        .arg(formal_dir.join("PointAddSolinasCuccaroAdder.tla"));
    run_command(config_name, &mut tla)
}

fn find_formal_dir() -> Option<PathBuf> {
    [PathBuf::from("../formal"), PathBuf::from("dev/formal")]
        .into_iter()
        .find(|path| path.join("PointAddSolinasCuccaroAdder.tla").exists())
}

fn lean_command() -> Command {
    if let Some(path) = env::var_os("LEAN") {
        return Command::new(path);
    }
    if let Some(profile) = env::var_os("USERPROFILE") {
        let direct = PathBuf::from(profile)
            .join(".elan")
            .join("toolchains")
            .join("leanprover--lean4---v4.30.0")
            .join("bin")
            .join("lean.exe");
        if direct.exists() {
            return Command::new(direct);
        }
    }
    let mut command = Command::new("elan");
    command.arg("run").arg("4.30.0").arg("lean");
    command
}

fn run_command(name: &str, command: &mut Command) -> bool {
    println!("formal_check_step={name}");
    match command.output() {
        Ok(output) => {
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            if output.status.success() {
                println!("formal_check_step={name} status=pass");
                true
            } else {
                eprintln!(
                    "formal_check_step={name} status=fail code={:?}",
                    output.status.code()
                );
                false
            }
        }
        Err(err) => {
            eprintln!("formal_check_step={name} status=fail error={err}");
            false
        }
    }
}

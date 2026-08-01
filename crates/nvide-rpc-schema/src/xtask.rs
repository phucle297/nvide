use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const CAPNP_VERSION: &str = "1.5.0";
const CAPNP_SHA256: &str = "d5ebdf858e9885c33d4b3f765006d68bd66e9b002bf4d607ff4317ef9c1aac6a";

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn Error>> {
    match (args.next().as_deref(), args.next().as_deref(), args.next()) {
        (Some("schema"), Some("generate"), None) => schema(false),
        (Some("schema"), Some("check"), None) => schema(true),
        (Some("evidence"), Some("check"), Some(flag)) if flag == "--phase" => {
            match (args.next().as_deref(), args.next()) {
                (Some("0"), None) => evidence_check(),
                _ => Err("usage: cargo xtask evidence check --phase 0".into()),
            }
        }
        _ => {
            Err("usage: cargo xtask {schema generate|schema check|evidence check --phase 0}".into())
        }
    }
}

fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut current = env::current_dir()?;
    loop {
        let manifest = current.join("Cargo.toml");
        if manifest.is_file() && fs::read_to_string(&manifest)?.contains("[workspace]") {
            return Ok(current);
        }
        if !current.pop() {
            return Err("workspace Cargo.toml was not found".into());
        }
    }
}

fn schema(check_only: bool) -> Result<(), Box<dyn Error>> {
    if !cfg!(target_os = "linux") {
        return Err("Phase 0 schema generation is supported on Linux only".into());
    }

    let root = repository_root()?;
    let compiler = ensure_compiler(&root)?;
    let schema = root.join("schemas/nrpc.capnp");
    let destination = root.join("crates/nvide-rpc-schema/src/generated/nrpc_capnp.rs");
    let temporary = root.join(format!(
        "target/phase0-tools/schema-output-{}",
        std::process::id()
    ));
    if temporary.exists() {
        fs::remove_dir_all(&temporary)?;
    }
    fs::create_dir_all(&temporary)?;

    let generation = (|| -> Result<Vec<u8>, Box<dyn Error>> {
        capnpc::CompilerCommand::new()
            .capnp_executable(&compiler)
            .src_prefix(root.join("schemas"))
            .file(&schema)
            .output_path(&temporary)
            .run()?;
        Ok(fs::read(temporary.join("nrpc_capnp.rs"))?)
    })();
    fs::remove_dir_all(&temporary)?;
    let generated = generation?;

    if check_only {
        if fs::read(&destination)? != generated {
            return Err(format!("generated schema drift: {}", destination.display()).into());
        }
        println!("schema check: clean");
    } else if fs::read(&destination).ok().as_deref() != Some(generated.as_slice()) {
        let parent = destination
            .parent()
            .ok_or("generated schema destination has no parent")?;
        fs::create_dir_all(parent)?;
        let staged = parent.join("nrpc_capnp.rs.tmp");
        fs::write(&staged, generated)?;
        if destination.exists() && cfg!(windows) {
            fs::remove_file(&destination)?;
        }
        fs::rename(staged, &destination)?;
        println!("generated {}", destination.strip_prefix(&root)?.display());
    } else {
        println!("schema generate: unchanged");
    }
    Ok(())
}

fn ensure_compiler(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let tools = root.join("target/phase0-tools");
    let compiler = tools.join("capnproto-build/c++/src/capnp/capnp");
    fs::create_dir_all(&tools)?;
    if compiler.is_file() && version_contains(&compiler, CAPNP_VERSION)? {
        println!("Cap'n Proto version {CAPNP_VERSION} (cached)");
        return Ok(compiler);
    }

    for (program, argument) in [
        ("curl", "--version"),
        ("sha256sum", "--version"),
        ("tar", "--version"),
        ("cmake", "--version"),
        ("c++", "--version"),
    ] {
        let output = checked_output(program, [argument], None)?;
        println!("{}", output.lines().next().unwrap_or(program));
    }

    let archive_name = format!("capnproto-v{CAPNP_VERSION}.tar.gz");
    let archive = tools.join(&archive_name);
    checked(
        "curl",
        [
            OsStr::new("--fail"),
            OsStr::new("--location"),
            OsStr::new("https://github.com/capnproto/capnproto/archive/refs/tags/v1.5.0.tar.gz"),
            OsStr::new("--output"),
            archive.as_os_str(),
        ],
        Some(&tools),
    )?;
    fs::write(
        tools.join("capnproto-v1.5.0.sha256"),
        format!("{CAPNP_SHA256}  {archive_name}\n"),
    )?;
    checked(
        "sha256sum",
        [OsStr::new("--check"), OsStr::new("capnproto-v1.5.0.sha256")],
        Some(&tools),
    )?;
    checked(
        "tar",
        [
            OsStr::new("--extract"),
            OsStr::new("--gzip"),
            OsStr::new("--file"),
            archive.as_os_str(),
            OsStr::new("--directory"),
            tools.as_os_str(),
        ],
        Some(root),
    )?;
    checked(
        "cmake",
        [
            OsStr::new("-S"),
            tools.join("capnproto-1.5.0").as_os_str(),
            OsStr::new("-B"),
            tools.join("capnproto-build").as_os_str(),
            OsStr::new("-DBUILD_TESTING=OFF"),
            OsStr::new("-DCMAKE_BUILD_TYPE=Release"),
        ],
        Some(root),
    )?;
    checked(
        "cmake",
        [
            OsStr::new("--build"),
            tools.join("capnproto-build").as_os_str(),
            OsStr::new("--config"),
            OsStr::new("Release"),
            OsStr::new("--target"),
            OsStr::new("capnp_tool"),
        ],
        Some(root),
    )?;
    if !version_contains(&compiler, CAPNP_VERSION)? {
        return Err(format!("{} is not Cap'n Proto {CAPNP_VERSION}", compiler.display()).into());
    }
    Ok(compiler)
}

fn version_contains(program: &Path, expected: &str) -> Result<bool, Box<dyn Error>> {
    Ok(checked_output(program, ["--version"], None)?.contains(expected))
}

fn checked<I, S>(
    program: impl AsRef<OsStr>,
    args: I,
    cwd: Option<&Path>,
) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with {status}").into())
    }
}

fn checked_output<I, S>(
    program: impl AsRef<OsStr>,
    args: I,
    cwd: Option<&Path>,
) -> Result<String, Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!("command failed with {}", output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn evidence_check() -> Result<(), Box<dyn Error>> {
    let root = repository_root()?;
    let plan = fs::read_to_string(root.join("docs/plan/README.md"))?;
    let ledger = fs::read_to_string(root.join("docs/evidence/phase-0.md"))?;
    let expected = expected_evidence();
    let definitions = phase_zero_definitions(&plan);
    let all_expected = expected
        .iter()
        .flat_map(|(row, references)| std::iter::once(*row).chain(references.iter().copied()))
        .collect::<BTreeSet<_>>();
    if definitions != all_expected {
        return Err(format!(
            "Phase 0 definition drift: expected {all_expected:?}, got {definitions:?}"
        )
        .into());
    }

    let mut rows = BTreeMap::new();
    for line in ledger.lines().filter(|line| line.starts_with("| P0-E")) {
        let cells = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.len() != 9 {
            return Err(format!("malformed evidence row: {line}").into());
        }
        let row = cells[0].trim_matches('`');
        if rows.insert(row, cells).is_some() {
            return Err(format!("duplicate evidence row {row}").into());
        }
    }

    for (row, required) in expected {
        let cells = rows
            .remove(row)
            .ok_or_else(|| format!("missing evidence row {row}"))?;
        let references = cells[2]
            .split(',')
            .map(|value| value.trim().trim_matches('`'))
            .filter(|value| !value.is_empty())
            .collect::<BTreeSet<_>>();
        let required = required.into_iter().collect::<BTreeSet<_>>();
        if references != required {
            return Err(format!("{row} references {references:?}; expected {required:?}").into());
        }
        if references.iter().any(|id| !all_expected.contains(id)) {
            return Err(format!("{row} contains an unknown ID").into());
        }
    }
    if !rows.is_empty() {
        return Err(format!("unknown evidence rows: {:?}", rows.keys()).into());
    }
    println!("Phase 0 evidence mapping: complete");
    Ok(())
}

fn phase_zero_definitions(plan: &str) -> BTreeSet<&str> {
    plan.lines()
        .flat_map(|line| {
            line.split(|character: char| {
                !(character.is_ascii_alphanumeric() || character == '-' || character == '.')
            })
        })
        .filter(|token| {
            matches!(
                *token,
                "P0.1"
                    | "P0.2"
                    | "P0.3"
                    | "P0-R1"
                    | "P0-R2"
                    | "P0-R3"
                    | "P0-R4"
                    | "P0-R5"
                    | "P0-R6"
                    | "P0-R7"
                    | "P0-A1"
                    | "P0-A2"
                    | "P0-A3"
                    | "P0-A4"
                    | "P0-A5"
                    | "P0-E1"
                    | "P0-E2"
                    | "P0-E3"
                    | "P0-E4"
                    | "P0-E5"
                    | "P0-E6"
            )
        })
        .collect()
}

fn expected_evidence() -> BTreeMap<&'static str, Vec<&'static str>> {
    BTreeMap::from([
        ("P0-E1", vec!["P0.1", "P0.2", "P0-R1", "P0-R2", "P0-A1"]),
        ("P0-E2", vec!["P0.3", "P0-R3", "P0-A2"]),
        ("P0-E3", vec!["P0-R4", "P0-A3"]),
        ("P0-E4", vec!["P0-R5", "P0-A4"]),
        ("P0-E5", vec!["P0-R6", "P0-A4"]),
        ("P0-E6", vec!["P0.2", "P0-R7", "P0-A5"]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_covers_all_phase_zero_ids() {
        let ids = expected_evidence()
            .into_iter()
            .flat_map(|(row, references)| std::iter::once(row).chain(references))
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 21);
    }
}

use std::process::Command;

/// Keys from `.env` that are made available to `env!()`. Variables set in the process environment take precedence.
const ENV_KEYS: &[&str] = &["BING_API_KEY", "OSM_CLIENT_ID", "TEST_ACCOUNT_TOKEN"];

fn main() {
	emit_git_info();
	emit_dotenv();
}

fn git(args: &[&str]) -> Option<String> {
	let output = Command::new("git").args(args).output().ok()?;
	output
		.status
		.success()
		.then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn emit_git_info() {
	println!("cargo:rerun-if-changed=.git/HEAD");
	println!("cargo:rerun-if-changed=.git/index");
	if let Some(head_ref) = git(&["symbolic-ref", "-q", "HEAD"]) {
		println!("cargo:rerun-if-changed=.git/{head_ref}");
	}

	let sha = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "0000000unknown".to_owned());
	let date = git(&["log", "-1", "--format=%cs"]).unwrap_or_else(|| "unknown".to_owned());
	let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());

	println!("cargo:rustc-env=VERGEN_GIT_SHA={sha}");
	println!("cargo:rustc-env=VERGEN_GIT_COMMIT_DATE={date}");
	println!("cargo:rustc-env=VERGEN_GIT_DIRTY={dirty}");
}

fn emit_dotenv() {
	println!("cargo:rerun-if-changed=.env");
	for key in ENV_KEYS {
		println!("cargo:rerun-if-env-changed={key}");
	}

	let Ok(contents) = std::fs::read_to_string(".env") else {
		return;
	};

	for line in contents.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let line = line.strip_prefix("export ").unwrap_or(line);
		let Some((key, value)) = line.split_once('=') else {
			continue;
		};
		let key = key.trim();
		if !ENV_KEYS.contains(&key) || std::env::var_os(key).is_some() {
			continue;
		}

		let value = value.trim();
		let value = ['"', '\'']
			.iter()
			.find_map(|q| value.strip_prefix(*q).and_then(|v| v.strip_suffix(*q)))
			.unwrap_or(value);
		println!("cargo:rustc-env={key}={value}");
	}
}

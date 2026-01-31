use std::env;
use std::fs;
use std::path::Path;

fn main() {
	let mut show_stats = false;
	let mut path: Option<String> = None;
	for arg in env::args().skip(1) {
		match arg.as_str() {
			"-s" | "--stats" => show_stats = true,
			"-h" | "--help" => {
				print_usage();
				return;
			}
			_ if path.is_none() => path = Some(arg),
			_ => {
				eprintln!("Unexpected argument: {arg}");
				print_usage();
				std::process::exit(2);
			}
		}
	}

	let Some(path) = path else {
		print_usage();
		std::process::exit(2);
	};

	let file_path = Path::new(&path);
	let file_name = file_path.file_name().and_then(|name| name.to_str()).unwrap_or(&path);

	let contents = match fs::read_to_string(file_path) {
		Ok(contents) => contents,
		Err(err) => {
			eprintln!("Failed to read {path}: {err}");
			std::process::exit(2);
		}
	};

	let (result, stats) = mappar_css::parse_str_with_stats(&contents, file_name);
	if show_stats {
		println!("Stats");
		println!("  rules:           {}", stats.rules);
		println!("  selector groups: {}", stats.selector_groups);
		println!("  selectors:       {}", stats.selectors);
		println!("  declarations:    {}", stats.declarations);
		println!("  styles:          {}", stats.style_decls);
		println!("  class assigns:   {}", stats.class_assigns);
		println!("  lex errors:      {}", stats.lex_errors);
		println!("  parse errors:    {}", stats.parse_errors);
	}

	if result.is_some() {
		println!("{file_name}: ok");
		return;
	}

	std::process::exit(1);
}

fn print_usage() {
	eprintln!("Usage: mapcss-validate [--stats] <file.mapcss>");
}

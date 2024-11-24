// extern crate vergen-gitcl;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let git = vergen_gitcl::GitclBuilder::all_git()?;
	vergen_gitcl::Emitter::default().add_instructions(&git)?.emit()?;

	Ok(())
}

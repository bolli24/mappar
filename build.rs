fn main() -> Result<(), Box<dyn std::error::Error>> {
	let git = vergen_gitcl::Gitcl::all().dirty(false).build();

	vergen_gitcl::Emitter::default().add_instructions(&git)?.emit()?;

	Ok(())
}

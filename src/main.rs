#![allow(dead_code)] // This is temporarily used for the inital development.

mod args;
mod fmt;
mod io;
mod runtime;
mod semantic;
mod symbol;
mod syntax;
mod term;
#[cfg(test)]
mod tests;

use term::color;

use args::{Args, Command};
use runtime::{Panic, SourcePos};


#[derive(Debug)]
enum ExitStatus {
	Success,
	InvalidArgs,
	StaticError,
	Panic,
}


impl From<ExitStatus> for i32 {
	fn from(status: ExitStatus) -> Self {
		match status {
			ExitStatus::Success => 0,
			ExitStatus::InvalidArgs => 1,
			ExitStatus::StaticError => 2,
			ExitStatus::Panic => 127,
		}
	}
}


fn main() -> ! {
	let command = match args::parse(std::env::args_os()) {
		Ok(command) => command,
		Err(error) => {
			eprint!("{}", error);
			std::process::exit(ExitStatus::InvalidArgs.into())
		}
	};

	let exit_status = match command {
		Command::Run(args) => run(args),
		Command::Help(msg) | Command::Version(msg) => {
			println!("{}", msg);
			ExitStatus::Success
		},
	};

	std::process::exit(exit_status.into())
}


fn run(args: Args) -> ExitStatus {
	let mut interner = symbol::Interner::new();
	let path = interner.get_or_intern("<stdin>");

	let source = match syntax::Source::from_reader(path, std::io::stdin().lock()) {
    Ok(source) => source,
    Err(error) => {
			eprintln!("{}", fmt::Show(Panic::io(error, SourcePos::file(path)), &interner));
			return ExitStatus::Panic;
		}
	};

	// ----------------------------------------------------------------------------------------
	let syntactic_analysis = syntax::Analysis::analyze(source, &mut interner);

	for error in syntactic_analysis.errors.iter().take(20) {
		eprintln!(
			"{}: {}",
			color::Fg(color::Red, "Error"),
			fmt::Show(error, &interner)
		);
	}

	if args.print_ast {
		println!("{}", color::Fg(color::Yellow, "--------------------------------------------------"));
		println!(
			"{}",
			fmt::Show(
				&syntactic_analysis.ast,
				syntax::ast::fmt::Context::from(&interner)
			)
		);
		println!("{}", color::Fg(color::Yellow, "--------------------------------------------------"));
	}

	// ----------------------------------------------------------------------------------------
	let program = match semantic::Analyzer::analyze(syntactic_analysis.ast, &mut interner) {
		Ok(program) => program,

		Err(errors) => {
			for error in errors.into_iter().take(20) {
				eprintln!(
					"{}: {}",
					color::Fg(color::Red, "Error"),
					fmt::Show(error, &interner)
				);
			}

			return ExitStatus::StaticError;
		}
	};

	if args.print_program {
		println!("{}", color::Fg(color::Yellow, "--------------------------------------------------"));
		println!(
			"{}",
			fmt::Show(
				&program,
				semantic::program::fmt::Context::from(&interner)
			)
		);
		println!("{}", color::Fg(color::Yellow, "--------------------------------------------------"));
	}

	// ----------------------------------------------------------------------------------------
	if !syntactic_analysis.errors.is_empty() {
		return ExitStatus::StaticError;
	}

	if args.check {
		return ExitStatus::Success;
	}

	let program = Box::leak(Box::new(program));

	match runtime::Runtime::eval(program, &mut interner) {
    Ok(_) => ExitStatus::Success,
    Err(panic) => {
			eprintln!("{}", fmt::Show(panic, &interner));
			ExitStatus::Panic
		}
	}
}

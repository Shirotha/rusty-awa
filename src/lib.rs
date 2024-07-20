use std::{
    fs::File,
    io::{stdin, stdout, BufReader, Error as IOError, IsTerminal, Read, Write},
    path::PathBuf,
    str::FromStr,
};

use awa_abyss::Abyss;
use awa_core::{
    load_awatalk, Assembler, AwaTism, BigEndian, BitError, BitReadBuffer, BitWriteStream,
    Endianness, ParseError, Program,
};
use awa_interpreter::{Cursor, Error as RuntimeError, FallibleIterator, Interpreter};

use clap::{Args, Parser, Subcommand, ValueEnum, ValueHint};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("coudn't infer file format, specify with the --format option")]
    UnknownFormat,
    #[error("can't read source code from a terminal input")]
    InputFromTerminal,
    #[error("failed to assemble program")]
    AssemblyFailed,
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error(transparent)]
    BitError(#[from] BitError),
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
    #[error(transparent)]
    IOError(#[from] IOError),
}

/// Format of the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
pub enum SourceFormat {
    /// use " Awa" and "wa" to represent bits (alias: awa)
    #[value(name = "awatalk", alias = "awa")]
    AwaTalk,
    /// assembly code (alias: tism)
    #[value(name = "awatism", alias = "tism")]
    AwaTism,
    /// bits packed into binary (alias: bin)
    #[value(alias = "bin")]
    Binary,
}
impl SourceFormat {
    #[inline]
    pub fn from_extension(name: impl AsRef<str>) -> Option<Self> {
        match name.as_ref() {
            "awa" => Some(Self::AwaTalk),
            "tism" => Some(Self::AwaTism),
            "bin" => Some(Self::Binary),
            _ => None,
        }
    }
}

/// Describes the location and format of the source code.
#[derive(Debug, Args)]
#[command(flatten = true)]
pub struct Source {
    /// Path to the file to diplay.
    ///
    /// Will try to guess the format based on file extension and header.
    /// Passing '-' will allow input to be piped from stdin, but format can not be guessed in that case.
    #[arg(
        value_name = "FILE",
        value_hint = ValueHint::FilePath
    )]
    file: PathBuf,
    /// Format of the source.
    ///
    /// When no format is given, a guess based on the context is made.
    #[arg(long, short = 'f', value_enum)]
    format: Option<SourceFormat>,
}
impl Source {
    pub fn read<E: Endianness>(&self) -> Result<Program, Error> {
        let mut buffer = Vec::new();
        let format = if self.file.to_str() == Some("-") {
            let mut handle = stdin();
            if handle.is_terminal() {
                return Err(Error::InputFromTerminal);
            }
            handle.read_to_end(&mut buffer)?;
            self.format.ok_or(Error::UnknownFormat)?
        } else {
            let mut handle = File::open(self.file.clone())?;
            handle.read_to_end(&mut buffer)?;
            self.format
                .or_else(|| SourceFormat::from_extension(self.file.extension()?.to_str()?))
                .or_else(|| {
                    if buffer[0..3].eq_ignore_ascii_case("awa".as_bytes()) {
                        Some(SourceFormat::AwaTalk)
                    } else {
                        None
                    }
                })
                .ok_or(Error::UnknownFormat)?
        };
        let program = match format {
            SourceFormat::AwaTalk => {
                let (raw, length) = load_awatalk::<E>(&buffer)?;
                Program::from_bitbuffer_with_length(raw, length)?
            }
            SourceFormat::AwaTism => {
                let mut assembler = Assembler::new();
                let (result, report) = assembler.assemble::<E>(buffer);
                report.print_all(&mut stdout(), assembler.fileserver(), true);
                let Some((raw, length)) = result else {
                    return Err(Error::AssemblyFailed);
                };
                Program::from_bitbuffer_with_length(raw, length)?
            }
            SourceFormat::Binary => {
                let raw = BitReadBuffer::new(&buffer, E::endianness());
                Program::from_bitbuffer(raw)?
            }
        };
        Ok(program)
    }
}

/// Describes compiler output location.
#[derive(Debug, Args)]
pub struct Out {
    /// Path of the output file.
    ///
    /// By default this will be derived by the input file.
    /// Passing '-' will allow output to be piped to stdout.
    #[arg(
        long, short = 'o',
        value_hint = ValueHint::FilePath
    )]
    out: Option<PathBuf>,
    /// Overwrite file if it already exists
    #[arg(long, short = 'F')]
    force: Option<bool>,
}
impl Out {
    pub fn write(&self, source: &Source, program: &Program) -> Result<(), Error> {
        let mut buffer = Vec::new();
        let mut writer = BitWriteStream::new(&mut buffer, BigEndian);
        for awatism in program {
            writer.write(awatism)?;
        }
        if self.out.as_ref().and_then(|f| f.to_str()) == Some("-") {
            let mut handle = stdout();
            handle.write_all(&buffer)?;
        } else {
            let mut out = self.out.as_ref().cloned().unwrap_or_else(|| {
                if source.file.to_str() == Some("-") {
                    PathBuf::from_str("out.bin").unwrap()
                } else {
                    source.file.with_extension("bin")
                }
            });
            if *source.file == out {
                out.set_extension("bin.bin");
            }
            let mut handle = if self.force.unwrap_or(false) {
                File::create(out)?
            } else {
                File::create_new(out)?
            };
            handle.write_all(&buffer)?;
        }
        Ok(())
    }
}

#[derive(Debug, Parser)]
#[command(about = "AWA CLI toolkit")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}
impl Cli {
    #[inline(always)]
    pub fn run(&self) -> Result<(), Error> {
        self.command.run()
    }
}
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Print file content as AwaTisms.
    #[command(arg_required_else_help = true)]
    Echo(Source),
    /// Build program from file or stdin.
    ///
    /// This will output data in the Binary format and can be ran using
    ///
    /// awa run --format binary out.bin
    #[command(arg_required_else_help = true)]
    Build {
        #[command(flatten)]
        source: Source,
        #[command(flatten)]
        output: Out,
    },
    /// Run program from file or stdin.
    #[command(arg_required_else_help = true)]
    Run {
        #[command(flatten)]
        source: Source,
        /// Print every instruction before it is executed
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    /// Debug program from file or stdin.
    ///
    /// Will advance one instruction at time while printing instructions and abyss.
    #[command(arg_required_else_help = true)]
    Debug {
        #[command(flatten)]
        source: Source,
    },
}
impl Commands {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Self::Echo(source) => {
                let program = source.read::<BigEndian>()?;
                let digits = (program.len() as f64).log10().trunc() as usize + 1;
                for (line, awatism) in program.into_iter().enumerate() {
                    // TODO: look ahead for prn instruction and print AWASCII chatacter instead of number
                    println!("{0:>1$} {2}", line + 1, digits, awatism)
                }
            }
            Self::Build { source, output } => {
                let program = source.read::<BigEndian>()?;
                output.write(source, &program)?;
            }
            Self::Run { source, verbose } => {
                let (program, abyss) = (source.read::<BigEndian>()?, Abyss::<isize>::new());
                let mut interpreter = Interpreter::new(abyss, BufReader::new(stdin()), stdout());
                if *verbose {
                    let digits = (program.len() as f64).log10().trunc() as usize + 1;
                    interpreter.run(&program).for_each(|(pc, awatism)| {
                        if matches!(awatism, AwaTism::Print) {
                            stdout().flush()?;
                            eprintln!();
                        }
                        eprintln!("{0:>1$} {2}", pc + 1, digits, awatism);
                        Ok(())
                    })?;
                } else {
                    interpreter.run(&program).last()?;
                }
            }
            Self::Debug { source } => {
                let (program, abyss) = (source.read::<BigEndian>()?, Abyss::<isize>::new());
                let mut interpreter = Interpreter::new(abyss, BufReader::new(stdin()), stdout());
                let mut cursor = Cursor::new(&program);
                let digits = (program.len() as f64).log10().trunc() as usize + 1;
                while let Some((pc, awatism)) = cursor.current() {
                    let str = format!("{}", interpreter.abyss());
                    for line in str.lines().rev() {
                        eprintln!("| {}", line);
                    }
                    eprint!("^\n{0:>1$} {2}\n> ", pc + 1, digits, awatism);
                    stdin().read_line(&mut String::new())?;
                    cursor.next(&mut interpreter)?;
                    if matches!(awatism, AwaTism::Print) {
                        stdout().flush()?;
                        eprintln!();
                    }
                }
            }
        }
        Ok(())
    }
}

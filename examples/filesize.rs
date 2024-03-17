//! Bash builtin to display file size.

use bash_builtins::{builtin_metadata, warning, Args, Builtin, BuiltinOptions, Error, Result};
use std::fs;
use std::io::{self, BufWriter, Write};

builtin_metadata!(
    name = "filesize",
    create = FileSize::default,
    short_doc = "filesize [args]",
    long_doc = "
        Display file sizes.

        Options:
          -k\tDisplay size in kilobytes.
          -m\tDisplay size in megabytes.

        Exit Status:
        Returns 0 if all files can be read; non-zero otherwise.
    ",
);

#[derive(Default)]
struct FileSize;

#[derive(BuiltinOptions)]
enum Opt {
    #[opt = 'k']
    Kilobytes,

    #[opt = 'm']
    Megabytes,
}

impl Builtin for FileSize {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        let mut scale = 1;

        for opt in args.options() {
            scale = match opt? {
                Opt::Kilobytes => 1 << 10,
                Opt::Megabytes => 1 << 20,
            }
        }

        let stdout_handle = io::stdout();
        let mut output = BufWriter::new(stdout_handle.lock());

        let mut result = Ok(());

        for path in args.path_arguments() {
            match fs::metadata(path) {
                Ok(m) => {
                    writeln!(&mut output, "{}\t{}", m.len() / scale, path.display())?;
                }

                Err(e) => {
                    warning!("{}: {}", path.display(), e);
                    result = Err(Error::ExitCode(1));
                }
            }
        }

        result
    }
}

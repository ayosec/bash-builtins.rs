//! Bash builtin to access shell variables.

use bash_builtins::variables::{self, Variable};
use bash_builtins::{builtin_metadata, Args, Builtin, Error, Result};
use std::io::{self, BufWriter, Write};

builtin_metadata!(name = "usevars", create = UseVars::default);

#[derive(Default)]
struct UseVars;

impl Builtin for UseVars {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        let stdout_handle = io::stdout();
        let mut output = BufWriter::new(stdout_handle.lock());

        for name in args.string_arguments() {
            let mut name_parts = name?.splitn(2, '=');
            match (name_parts.next(), name_parts.next()) {
                (Some(name), None) => {
                    if name.contains('[') {
                        get_array(name)?;
                    } else {
                        match variables::find(name) {
                            Some(var) => write_var(&mut output, name, var)?,
                            None => (),
                        }
                    }
                }

                (Some(name), Some("")) => {
                    if variables::unset(name) {
                        writeln!(&mut output, "unset: {}", name)?;
                    }
                }

                (Some(name), Some(value)) => {
                    if name.contains('[') {
                        set_array(name, value)?;
                    } else {
                        variables::set(name, value)?;
                    }
                }

                _ => (),
            }
        }

        Ok(())
    }
}

fn write_var(mut output: impl Write, name: &str, var: Variable) -> io::Result<()> {
    match var {
        Variable::Str(s) => writeln!(&mut output, "{} = {:?}", name, s)?,

        Variable::Array(a) => {
            for (idx, elem) in a.iter().enumerate() {
                writeln!(&mut output, "{}[{}] = {:?}", name, idx, elem)?;
            }
        }

        Variable::Assoc(a) => {
            for (key, value) in a.iter() {
                writeln!(&mut output, "{}[{:?}] = {:?}", name, key, value)?;
            }
        }
    }

    Ok(())
}

fn parse_array_ref(name: &str) -> Result<(&str, &str)> {
    let (open, close) = match (name.find('['), name.find(']')) {
        (Some(a), Some(b)) if b + 1 == name.len() => (a, b),
        _ => Err(Error::Usage)?,
    };

    let var_name = &name[..open];
    let key = &name[open + 1..close];

    Ok((var_name, key))
}

fn get_array(name: &str) -> Result<()> {
    let (var_name, key) = parse_array_ref(name)?;

    let value = if let Ok(index) = key.parse() {
        variables::array_get(var_name, index)
    } else {
        variables::assoc_get(var_name, key)
    };

    println!("{} = {:?}", name, value);

    Ok(())
}

fn set_array(name: &str, value: &str) -> Result<()> {
    let (var_name, key) = parse_array_ref(name)?;

    if let Ok(index) = key.parse() {
        variables::array_set(var_name, index, value)?;
    } else {
        variables::assoc_set(var_name, key, value)?;
    }

    Ok(())
}

A derive macro to generate a command-line arguments parser for
[`Args::options`]. The parser uses the `getopt()` implementation provided by
bash.

The macro is applied only to [enumerations]. Each variant is an option accepted
by the builtin. The letter for the option is set with the `#[opt = '…']`
attribute.

[enumerations]: https://doc.rust-lang.org/reference/items/enumerations.html

# Example

```ignore
// Options parser.

#[derive(BuiltinOptions)]
enum Opt<'a> {
    #[opt = 'v']
    Verbose,

    #[opt = 'l']
    Limit(u16),

    #[opt = 'n']
    Name(&'a str),

    #[opt = 'w']
    Write(Option<&'a std::path::Path>),
}


// Builtin implementation.

struct Foo;

impl Builtin for Foo {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        let mut verbose = false;
        let mut limit = None;
        let mut name = None;
        let mut write = None;

        for opt in args.options() {
            match opt? {
                Opt::Verbose => verbose = true,
                Opt::Limit(l) => limit = Some(l),
                Opt::Name(n) => name = Some(n),
                Opt::Write(w) => write = w,
            }
        }

        // …

        Ok(())
    }
}
```

# Option Arguments

Options can have one argument. The value for that argument is taken from the
next word in the command-line.

If the type of the argument is an [`Option<T>`], then the option can be used
without an argument. Bash assumes a missing argument if the option is the last
argument of the command-line, or the next argument starts with a hyphen.

For example, if the builtin defined in the [previous example](#example) is
executed with the following arguments:

```notrust
$ foo -l 100 -w -w name
```

The iterator from `args.options()` yields three items:

```ignore
Ok(Opt::Limit(100))

Ok(Opt::Write(None))

Ok(Opt::Write(Some(Path("name"))))
```

The type of the argument requires the `FromWordPointer` implementation, which
is provided by default for many types of the standard library.

## Error Handling

In the [previous example](#example), the variant `Opt::Limit` is associated with
the option `-l`, and it requires an argument of type `u16`. If the argument is
missing when the builtin is invoked, or its value can't be converted to `u16`,
then an error is printed to *stderr*, and the `args.options()` iterator yields
an error. This error is propagated to the caller in the `match opt?` line of
the example.

# Using References

For [`str`], [`Path`](std::path::Path), and [`OsStr`](std::ffi::OsStr), it is
possible to get a reference instead of an owned value. Its lifetime is bound to
the `&mut Args` variable received as an argument of the `call` function, so it
is possible to use it inside the function, but the value needs to be cloned if
it is stored for future calls.

[`Args::options`]: struct.Args.html#method.options
[`Option<T>`]: std::option::Option

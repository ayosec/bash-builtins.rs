Macro to generate the code to define a new builtin for bash.

# Usage

```notrust
builtin_metadata!(
    name = string literal,
    create | try_create = path,
    short_doc = string literal,
    long_doc = string literal,
);
```

Arguments are specified as `key = value` items, where `key` can be:

* `name`.

    The name of the builtin.

* `create` or `try_create`.

    A [path] to a function to initialize the builtin.

    Just one of these keys has to be present.

* `short_doc` and `long_doc`.

    Optional keys for the builtin documentation.

See below for more details.

The generated code requires the [`bash_builtins`] crate to be available in the
package.

# Example

A builtin like [`eval`] can be defined with the following arguments:

```ignore
builtin_metadata!(
    name = "eval",
    create = Eval::default,
    short_doc = "eval [arg ...]",
    long_doc = "
        Execute arguments as a shell command.

        Combine ARGs into a single string, use the result as input to the shell,
        and execute the resulting commands.

        Exit Status:
        Returns exit status of command or success if command is null.
    ",
);
```

`Eval::default` is a [path] to a function that returns an instance of the
[`Builtin`] trait.

# Arguments

## `name`

The name of the builtin. It is required to be an ASCII identifier.

Users will type this name to invoke the builtin defined by the macro.

## `create`

A [path] to a function that returns an instance of the [`Builtin`] trait.

See below for more details on how to define that function.

## `try_create`

Similar to `create`, but the function returns an instance of
`Result<T: Builtin, E: Display>`. If the function returns [`Err`],
the builtin will not be loaded.

## `short_doc` (optional)

A single line of text to describe how to use the builtin.

This is optional, but it is recommended.

## `long_doc` (optional)

Documentation of the builtin. Bash uses this text for the [`help`] command.
Like `short_doc`, it is optional but recommended.

The content of the text is processed to remove the leading new lines, and the
left margin introduced to indent the content.

These two `long_doc` expressions are equivalent:

```ignore
// help text for the `true` builtin.

# let
long_doc = "
    Return a successful result.

    Exit Status:
    Always succeeds.
"

# ; let
long_doc = "Return a successful result.\n\nExit Status:\nAlways succeeds.\n"
# ;
```

This transformation is similar to the [indoc](https://docs.rs/indoc)
crate, the [text blocks](https://openjdk.java.net/jeps/378) in Java,
[“squiggly” heredocs] in Ruby, and many others.

# Builtin Initialization

Builtins are implemented as instances of the [`Builtin`] trait. To create
that instance, one of `create` or `try_create` is required in the
`builtin_metadata!()` macro.

The function given in `create` returns the instance. In the following example
we use the generated [`default`] function to create a new instance of the
`Counter` type.

```ignore
use bash_builtins::{builtin_metadata, Args, Builtin, Result};

builtin_metadata!(
    name = "counter",
    create = Counter::default,
    // …
);

#[derive(Default)]
struct Counter(isize);

impl Builtin for Counter {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        // …
    }
}
```

The function given in `try_create` returns a `Result<T, E>`, where
`T` is the type for the builtin, and `E` implements [`Display`]. If the
function returns an error then the builtin will not be loaded.

In the following example, the builtin will always fail to load:

```ignore
builtin_metadata!(
    name = "loadfail",
    try_create = try_create,
    // …
);

fn try_create() -> std::result::Result<LoadFail, &'static str> {
    Err("something really bad happened")
}

struct LoadFail;

impl Builtin for LoadFail {
    fn call(&mut self, _args: &mut Args) -> Result<()> {
        Ok(())
    }
}
```

We can compile this example and then try to load it:

```notrust
$ cargo build --release --examples

$ enable -f target/release/examples/libloadfail.so loadfail
loadfail: error: something really bad happened
bash: enable: load function for loadfail returns failure (0): not loaded
```

[`Builtin`]: trait.Builtin.html
[`Display`]: ::std::fmt::Display
[`Err`]: std::result::Result::Err
[`bash_builtins`]: https://docs.rs/bash_builtins
[`default`]: ::std::default::Default::default
[`eval`]: https://www.gnu.org/software/bash/manual/html_node/Bourne-Shell-Builtins.html#index-eval
[`help`]: https://www.gnu.org/software/bash/manual/html_node/Bash-Builtins.html#index-help
[path]: https://doc.rust-lang.org/reference/paths.html
[“squiggly” heredocs]: https://docs.ruby-lang.org/en/3.0.0/doc/syntax/literals_rdoc.html#label-Here+Documents+-28heredocs-29

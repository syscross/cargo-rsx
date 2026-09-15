
# RSX

**Rust Extended.** A thin layer on top of Rust that lets you write
initialization in a more C++-like order, then rewrites it into real,
idiomatic Rust before `rustc` ever sees it.

RSX doesn't relax what Rust requires. It just lets you satisfy that
requirement a little later than Rust normally allows.

## The idea

Rust wants this:
```rust
let a = Hello { name: "RSX".to_string() };
```

C++ habits want this:
```rust
let a: Hello;
a.name = "RSX".to_string();
```

Normally, the second form doesn't compile — `rustc` rejects
partially-initialized bindings outright (`E0381`). RSX lets you write
it that way anyway. Behind the scenes, it rewrites the sequence into
the first form before compiling. The `.rs` file `rustc` actually sees
is fully idiomatic — no `unsafe`, no `MaybeUninit`, nothing hidden.

## What works now

### Deferred field initialization

```rust
struct Hello {
    name: String,
}

impl Hello {
    fn hello(&self) {
        println!("Hello {}", self.name);
    }
}

fn main() {
    let a: Hello;
    a.name = "RSX".to_string();

    a.hello();
}
```

This compiles under RSX. Internally it becomes:

```rust
let a = Hello { name: "RSX".to_string() };
```

### Compile-time check: all fields must be filled before use

If a field is missing when the value is first used, RSX stops before
`rustc` even runs:

```rust
struct Hello {
    name: String,
    desc: String,
}

fn main() {
    let a: Hello;
    a.name = "RSX".to_string();

    a.hello();  // desc was never set
}
```

```
error: use of possibly-uninitialized value `a` — missing field(s): `desc`
```

**The requirement is identical to real Rust.** Every field still has
to be set before the value is used — RSX just lets you do it across
several lines instead of one expression.

## Why this exists

This isn't about making Rust "easier" or "less strict." The safety
guarantee stays exactly as strong. What changes is the *shape* of the
code that gets you there — RSX allows the same step-by-step,
declare-then-fill style that's second nature in C++, and turns it
into whatever real Rust actually wants underneath.

If you're moving a C++ codebase toward Rust (see the companion
project, [`cargo-txx`](../cargo-txx)), RSX is meant to be the
landing spot right before pure `.rs` — familiar enough to write
without a mental context switch, strict enough that what compiles is
real, safe Rust.

## Known limits

- Only handles field assignments that appear right after the `let`
  declaration, on their own lines — assignments inside `if`/`match`
  branches aren't tracked yet.
- Comments are stripped with a simple `//` scan, so `//` inside a
  string literal can be misread as a comment.
- Regex + brace-matching, not a real parser.

## Usage

```bash
cargo run -- path/to/file.rsx
```

Writes the rewritten `.rs` and compiled binary to `target/rsx/`, then
runs it.
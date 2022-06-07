### Cheshire, an R5RS-compatible Scheme interpreter

This is my take on an R5RS-compatible Scheme interpreter, loosely based off the wonderful [Crafting Interpreters](https://craftinginterpreters.com/) book by Robert Nystrom.
Most of the required standard library functions are present, but it's still in a somewhat primitive state.
Cheshire is not production-ready - use at your own risk.

#### Running

To run Cheshire, simply use `cargo run --release` to spawn a REPL.

```
$ cargo run --release
<...compiling...>
>> 
```

Use the `debug-trace-execution` feature flag to print out currently executing bytecode instructions and stack info.

```
$ cargo run --release --features debug-trace-execution
```

You can also use the builtin `disassemble` procedure to introspect a procedure's bytcode.

#### Bugs/missing features

- `call/cc` does not save stack state correctly - it needs to box the contents of the captured stack and make a fixed-size copy of it.
  - This involves some runtime overhead however, and it seems like a smarter compiler could avoid (some of) this.
- Cheshire uses so-called "upvalues" to capture closed-over variables, but doesn't completely implement them.
  - Currently, the entire enclosing stack is captured, rather than the single value that's being closed over.
  - This would be a fairly easy optimization to implement.
- `dynamic-wind` is currently unimplemented.
- Syntax macros and quasiquoting are currently unimplemented within the bootstrap compiler (but would be fairly easy to implement within scheme itself).
- Support for recording line info is present, but isn't really used since the reader doesn't propagate line info right now.
  - It would be pretty easy to propagate line info (the parsing library being used emits it), but it would significantly complicate the AST.
  - Since we don't have destructuring/pattern matching within the bootstrap compiler, this makes things _very_ messy.
- Read-only and write-only ports are implemented, but there is no support for read/write ports (yet).
- Garbage collection is overly pessimistic and is run a bit too often.
- It should be possible to allocate procedure stack size up front rather than use a growable stack, but this isn't done (yet).
- There is no builtin `eval` procedure, but it is fairly easy to build a poor-man's version by wrapping the builtin `compile` procedure.
- Loading scripts directly has some issues, but you can launch a REPL and use the `load` builtin to load a file.
- The only supported numeric type is currently `f64` (a double-precision float).  It would be nice to have the rest of the numeric tower and arbitrary-precision numbers.
- Symbols are interned (yay) into a global symbol table (D:).  There is currently no way to evict "dead" (unused) symbols from this table, so these will leak memory over time.
  - It should be possible to implement function-local symbol tables (so they can be released when the procedure goes out of scope), but supporting `eval` makes things annoying.

#### Implementation details

Cheshire is written in Rust and currently tries to avoid `unsafe` where possible.  In the future more `unsafe` code may creep in to allow for better optimizations.

##### Data structures

There are three main data structures that are used by Cheshire: `Datum`s, `Value`s, and `Object`s.

`Datum`s are constant data emitted by the reader, and can represent common values like characters, symbols, or numbers but can also point to data structures like lists/vectors/strings.
`Value`s are like `Datum`s, but can also represent mutable (boxed) values, and are the primary runtime type used in the bytecode VM.
`Object`s are things that live on the heap (boxed values), or in other words, things that are too large to fit within a small (~16 byte) `Value`.  This includes things like vectors, strings, procedures, etc.

All of these data structures are represented as pure Rust `enum`s, no bit twiddling or bit packing is involved (yet).

There is also a `VirtualMachine` struct, which is primarily used as a GC root and to record state between garbage collections.  You can think of this as a special kind of continuation - i.e. the current continuation.

##### Garbage collection

Cheshire doesn't implement its own garbage collector, rather it relies on the [gc-arena crate](https://crates.io/crates/gc-arena).  This has some limitations however:

- gc-arena doesn't allow unsized types to live behind a GC pointer, meaning there are a lot of double-indirections where only one should be necessary.
- gc-arena implements a simple mark-and-sweep algorithm and requires a stop-the-world approach to garbage collection.
  - It would be nice if the GC algorithm could be made pluggable.  It's at least tunable with the current release.

Cheshire itself is also a bit stupid with its use of garbage collection.  GC happens upon every procedure call, tail call, or return.  While this guarantees there is no unbounded memory growth, this is far too often and does have a measureable performance impact.
A better strategy would be allocate a fixed-size stack for use as a cheap bump allocator (or perhaps use something like [bumpalo](https://crates.io/crates/bumpalo)) and hand out all objects from its memory.
If the stack runs out of space, do a "soft" GC to move live objects to the heap and clean out any garbage left on the stack.  If there's no heap space left, only then do we do a full GC.
This stack-like data structure is often called a _nursery_ in PL lingo.

##### Bytecode

Cheshire implements a stack-like VM with a subset of the bytecode outlined in the second half of the Crafting Interpreters book.  It has a few quirks, notably:

- There are no native arithmetic instructions - all arithmetic operations are reduced to function calls, there is no inlining done by the bootstrap compiler (by design - the bootstrap compiler is meant to be dead simple).
- There is no native instruction for jumping backwards within a bytecode block.
  - This means there is no way to make a non-trivial loop within a bytecode block.  The only way to loop is to perform a (tail) call.
  - One advantage of this approach is that it makes it possible to calculate the maximum stack size for a given function invokation, which means its stack could be pre-allocated with a fixed-size array (rather than a growable one).
- The most complicated instruction by far is the `CLOSURE` instruction, which constructs a closure that captures variables from a surrounding scope.
- Most other instructions are simply loads or stores that manipulate the stack.

The total number of instructions is quite small (~20 total, although some are not totally necessary), and this was done deliberately to keep things simple (if somewhat suboptimal/slow).
Adding specialized instructions (e.g. arithmetic, special conditional logic, etc.) is (typically) an optimization, which will be pursued at a later date.
The bootstrap compiler doesn't do any control flow analysis or tail call elimination (so no optimizations, even easy ones like constant folding), but does detect when a tail call can be performed and emits a `TAIL_CALL` instruction (this is required by the Scheme spec).
The compiler is available at runtime under the `compile` builtin procedure.

#### Future plans

- (Re-)implement the bootstrap compiler in scheme to self host
- Implement optimizations
- Support for syntax macros
- Experiment with native (JIT) compilation
- Explore libuv + libffi (or Rust equivalents) instead of implementing our own event loop and ABI
- Integration with debuggers? (gdb/lldb)
- SRFI support
- Support newer RNRS features beyond R5RS

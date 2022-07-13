## CLI for zkMove Lite

Currently, we only support one command 'Run', which run the full sequence of circuit building, setup, proving, and verifying.
It also reports the proof size, prove time and verify time when the execution is successful.

For example, the following command will first compile add.move into bytecode, execute the bytecode to generate an execution trace,
then build the circuit and setup the proving/verifying key, and then generate a zkp for the execution with the proving key and
finally verify the proof with the verifying key.

```bash
zkmove run -s examples/scripts/add.move
```

### Import modules
The Move program consists of scripts and modules. For testing, directive 'mods' can be added to script source file to import a module. For example,

```rust
/// call_u8.move

//! mods: arith.move
//! args: 1u8, 2u8
script {
    use 0x1::M;
    fun main(x: u8, y: u8) {
        M::add_u8(x, y);
    }
}
```
And we need tell vm where to load the module with option '-m':

```bash
zkmove run -s examples/scripts/call_u8.move -m examples/modules/
```
### Pass arguments
As you may have noticed, directive 'args' is used to pass arguments to scripts. There is also a command
option '--new-args' can be used to pass arguments, but these two methods have different purpose.

When using '--new-args', vm first runs the script with the old arguments (set by the directive 'args') and generates the
proving/verifying keys. Then, the script is run with the new arguments and the zkp is generated/verified with the **old** proving/verifying
keys. For example,

```rust
/// add_u8.move

//! args: 1u8
script {
    fun main(x: u8) {
        x + 2u8;
    }
}
```

```bash
zkmove run -s examples/scripts/add_u8.move --new-args 2u8
```

<div align="center">
<h1>Arisca</h1>
<p>
<strong>Formal Verification for Arithmetic Circuits via Symbolic Computer Algebra</strong>
</p>
<img src="assets/logo.png" width="90%" alt="Arisca Banner">

</div>
Arisca is a formal verification tool designed to rigorously prove the correctness of arithmetic circuit units. It employs Symbolic Computer Algebra (SCA) and polynomial elimination techniques to mathematically verify that a gate-level circuit implementation matches its arithmetic specification.

## 📦 Build
Ensure you have the Rust toolchain installed.
```bash
# Clone the repository
git clone https://github.com/LittleBlackCQ/arisca.git
cd arisca

# Build the release binary
cargo build --release
```
## 🚀 Usage
Arisca processes AIGER (.aig) files. You can execute it directly using cargo run.

### Basic Verification
To verify an arithmetic circuit against its specification:
```bash 
cargo run --bin arisca -- <AIG_FILE> [-d <DOT_FILE> -l <LOG_FILE>]
```


## 📄 License
See the LICENSE file for details.
# envcaja

**envcaja** is a file-based secret manager designed for CI/CD pipelines.
It can encrypt a single .env file or a whole path of files.
You can supply the key as environment variable or as a key file.

---

## Installation

Build from source with Cargo:

```bash
cargo build --release
```

The binary will be available in `target/release/envcaja`.

---

## Usage

```bash
envcaja [OPTIONS] <COMMAND>
```

### Options

* `-v, --verbose` : Increase verbosity. Can be repeated (`-v`, `-vv`, `-vvv`)
* `--keyfile <PATH>` : Path to the keyfile (default: `safe.key`)
* `--key <KEY>` : the secret key. This is equal to setting CI_SECRET environment variable
* `--env-conf <PATH>` : Path to the secret environment configuration. Can be a file or a folder
* `--vault <PATH>` : Path to the encrypted vault file.

### Commands

#### `init`

Initializes the repository:

```bash
envcaja init
```

### Options

* `--folder` : Create a folder instead of a single configuration file

Generates a new key and saves it in the keyfile.
Updates `.gitignore` to exclude secret files.
Creates an empty .env file or folder.

#### `info`

Analyzes the repository and checks for configuration errors.
Displays all important informations.

```bash
envcaja info
```

#### `encrypt`

Encrypts a `.env` file or directory:

```bash
envcaja encrypt 
```

#### `decrypt`

Decrypts an encrypted vault back to a file or folder:

```bash
envcaja decrypt
```

---

## Environment Variable

You can provide the key via the `CI_SECRET` environment variable:

```bash
export CI_SECRET="your_base64_key_here"
envcaja encrypt 
```

If no key is provided, the program will use the default keyfile (`safe.key`).


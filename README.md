# rustssh
rust ssh sudo password scp

## Usage

#### 1. add dependencies
```
# Cargo.toml
[dependencies]
rustssh = "0.1.2"
```

#### 2. edit code
```
# src/main.rs

use rustssh::{Connection, Auth, SudoOptions};

fn main() {

    // create connection
    let mut conn = Connection::new("1.1.1.1".to_string(), 22, "username".to_string(), Auth::Password("password".to_string())).unwrap();

    // Execute remote commands as the connected user
    let (stdout, stderr, status) = conn.run("ls").unwrap();
    println!("The output of run is: {}, The outerr of run is: {}, The status code of run is: {}, end.", stdout, stderr, status);

    // Execute remote commands as root user
    let (stdout, stderr, status) = conn.sudo("ls -l /root/").unwrap();
    println!("The output of sudo is: {}, The outerr of sudo is: {}, The status code of sudo is: {}, end.", stdout, stderr, status);

    // Execute remote commands as user user01
    // SudoOptions::new() Specify three parameters: 1. The username to sudo, 2. The password that needs to be entered when sudo, 3. Specify the sudo prompt
    let opt = SudoOptions::new("user01", "", "");
    let (stdout, stderr, status) = conn.sudo_with_options("ls -l /home/user01/", Some(opt)).unwrap();
    println!("The output of sudo is: {}, The outerr of sudo is: {}, The status code of sudo is: {}, end.", stdout, stderr, status);

    // scp copies local files to remote
    conn.scp("/home/xxx/x.xml", "/home/xxx/xx.xml").unwrap();
}
```

## Features (see future work needs)

- conn.scp() can copy directories
- is_exists()
- conn.is_dir()
- conn.is_empty()
- ...

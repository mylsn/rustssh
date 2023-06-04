# rustssh
rust ssh sudo password scp

## Usage

#### 1. add dependencies
```
# Cargo.toml
[dependencies]
rustssh = "0.1.0"
```

#### 2. edit code
```
# src/main.rs
        let mut conn = Connection::new(myhost, port, myusername, Auth::Password(mypassword)).unwrap();

        let (stdout, stderr, status) = conn.run("ls").unwrap();
        println!("The output of run is: {}, The outerr of run is: {}, The status code of run is: {}, end.", stdout, stderr, status);

        let (stdout, stderr, status) = conn.sudo("ls -l /root/").unwrap();
        println!("The output of sudo is: {}, The outerr of sudo is: {}, The status code of sudo is: {}, end.", stdout, stderr, status);

        conn.scp("/home/xxx/x.xml", "/home/xxx/xx.xml").unwrap();
```

## Features (see future work needs)

- conn.scp() can copy directories
- is_exists()
- conn.is_dir()
- conn.is_empty()
- ...
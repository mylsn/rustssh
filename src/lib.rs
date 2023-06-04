//! # rustssh
//! rust ssh sudo password scp
//! 
//! ## Usage
//! 
//! #### 1. add dependencies
//! ```
//! # Cargo.toml
//! [dependencies]
//! rustssh = "0.1.2"
//! ```
//! 
//! #### 2. edit code
//! ```
//! # src/main.rs
//! 
//! use rustssh::{Connection, Auth, SudoOptions};
//! 
//! fn main() {
//! 
//!     // create connection
//!     let mut conn = Connection::new("1.1.1.1".to_string(), 22, "username".to_string(), Auth::Password("password".to_string())).unwrap();
//! 
//!     // Execute remote commands as the connected user
//!     let (stdout, stderr, status) = conn.run("ls").unwrap();
//!     println!("The output of run is: {}, The outerr of run is: {}, The status code of run is: {}, end.", stdout, stderr, status);
//! 
//!     // Execute remote commands as root user
//!     let (stdout, stderr, status) = conn.sudo("ls -l /root/").unwrap();
//!     println!("The output of sudo is: {}, The outerr of sudo is: {}, The status code of sudo is: {}, end.", stdout, stderr, status);
//! 
//!     // Execute remote commands as user user01
//!     // SudoOptions::new() Specify three parameters: 1. The username to sudo, 2. The password that needs to be entered when sudo, 3. Specify the sudo prompt
//!     let opt = SudoOptions::new("user01", "", "");
//!     let (stdout, stderr, status) = conn.sudo_with_options("ls -l /home/user01/", Some(opt)).unwrap();
//!     println!("The output of sudo is: {}, The outerr of sudo is: {}, The status code of sudo is: {}, end.", stdout, stderr, status);
//! 
//!     // scp copies local files to remote
//!     conn.scp("/home/xxx/x.xml", "/home/xxx/xx.xml").unwrap();
//! }
//! ```
//! 
//! ## Features (see future work needs)
//! 
//! - conn.scp() can copy directories
//! - is_exists()
//! - conn.is_dir()
//! - conn.is_empty()
//! - ...

use ssh2::{self, Session};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
// use ssh2::PtyModes;

struct Watcher {
    pattern: String,  // 要捕获字符串
    response: String, // 捕获到匹配的字符串出现后, 要输入的内容
    // sentinel: String, // TODO 应答后返回的信息与之匹配, 说明应答失败. 还未实现
    case_sensitive: bool, // 是否区别大小写
}

pub struct RunOptions {
    watchers: Vec<Watcher>,
}

impl RunOptions {
    pub fn new() -> RunOptions {
        RunOptions { watchers: Vec::new() }
    }

    pub fn set_watcher(&mut self, pattern: &str, response: &str, case_sensitive: bool) {
        self.watchers.push(Watcher { pattern: pattern.to_string(), response: response.to_string(), case_sensitive });
    }
}

pub struct SudoOptions {
    sudo_user: String,
    sudo_password: String,
    sudo_pattern: String,
    run: RunOptions,
}

impl SudoOptions {
    pub fn new(sudo_user: &str, sudo_password: &str, sudo_pattern: &str) -> SudoOptions {
        SudoOptions {
            sudo_user: sudo_user.to_string(),
            sudo_password: sudo_password.to_string(),
            sudo_pattern: sudo_pattern.to_string(),
            run: RunOptions { watchers: Vec::new() },
        }
    }

    pub fn set_watcher(&mut self, pattern: &str, response: &str, case_sensitive: bool) {
        self.run.watchers.push(Watcher { pattern: pattern.to_string(), response: response.to_string(), case_sensitive });
    }
}

pub enum Auth {
    Password(String),
    Privatekey(String),
    Privatekeyfile(String),
}

impl Clone for Auth {
    fn clone(&self) -> Self {
        match self {
            Self::Password(arg0) => Self::Password(arg0.clone()),
            Self::Privatekey(arg0) => Self::Privatekey(arg0.clone()),
            Self::Privatekeyfile(arg0) => Self::Privatekeyfile(arg0.clone()),
        }
    }
}

pub struct Connection {
    host: String,
    port: u16,
    user: String,
    auth: Auth,
    timeout: u32,
    session: Session,
}

impl Connection {
    pub fn new(host: String, port: u16, user: String, auth: Auth) -> Result<Connection, Box<dyn Error>> {
        let timeout: u32 = 60000;

        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(addr)?;

        let mut session = Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();
        session.set_timeout(timeout);

        let mut conn = Connection {
            host,
            port,
            user,
            auth,
            timeout,
            session,
        };
        conn.authenticated()?;
        conn.session.authenticated();

        Ok(conn)
    }

    fn authenticated(&mut self) -> Result<(), ssh2::Error> {
        match &self.auth {
            Auth::Password(password) => self.session.userauth_password(&self.user, &password),
            Auth::Privatekey(privatekey) => self.session.userauth_pubkey_memory(&self.user, None, &privatekey, None),
            Auth::Privatekeyfile(privatekey_file) => {
                let privatekey_path = Path::new(&privatekey_file);
                self.session.userauth_pubkey_file(&self.user, None, privatekey_path, None)
            }
        }
    }

    pub fn get_host(&mut self) -> String {
        self.host.to_string()
    }

    pub fn get_port(&mut self) -> u16 {
        self.port
    }

    pub fn get_user(&mut self) -> String {
        self.user.to_string()
    }

    pub fn get_auth(&mut self) -> Auth {
        self.auth.clone()
    }

    pub fn get_timeout(&mut self) -> u32 {
        self.timeout
    }

    pub fn set_timeout(&mut self, timeout: u32) {
        self.timeout = timeout;
        self.session.set_timeout(self.timeout);
    }

    fn run_watcher(&mut self, channel: &mut ssh2::Channel, watchers: Vec<Watcher>, stdout: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
        loop {
            let mut buf = [0; 1024];

            let n = channel.read(&mut buf)?;
            if n == 0 {
                break;
            }

            let slice = &buf[0..n];
            stdout.extend_from_slice(slice);

            let s = String::from_utf8_lossy(slice);

            for w in watchers.iter() {
                if w.case_sensitive {
                    if s.contains(&w.pattern) {
                        channel.write_all(format!("{}\n", w.response).as_bytes())?;
                        channel.flush()?;
                    }
                    
                } else {
                    if s.to_uppercase().contains(&w.pattern.to_uppercase()) {
                        channel.write_all(format!("{}\n", w.response).as_bytes())?;
                        channel.flush()?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn run_with_options(&mut self, cmd: &str, options: Option<RunOptions>) -> Result<(String, String, i32), Box<dyn Error>> {
        let command = format!("PATH=$PATH:/usr/bin:/usr/sbin {}", cmd);

        let mut channel = self.session.channel_session()?;

        // let mut modes = PtyModes::new();
        // modes.set_boolean(ssh2::PtyModeOpcode::TCSANOW, false);
        // modes.set_boolean(ssh2::PtyModeOpcode::ECHO, false);
        // modes.set_u32(ssh2::PtyModeOpcode::TTY_OP_ISPEED, 144000);
        // modes.set_u32(ssh2::PtyModeOpcode::TTY_OP_OSPEED, 144000);
        // channel.request_pty("xterm", Some(modes),Some((80, 40, 0, 0))).unwrap();

        channel.handle_extended_data(ssh2::ExtendedData::Merge)?;
        channel.exec(&command)?;

        let mut stdout = Vec::new();

        match options {
            Some(opt) => {
                if opt.watchers.len() != 0 {
                    self.run_watcher(&mut channel, opt.watchers, &mut stdout)?
                }
            }
            None => (),
        }

        let mut stdout = String::from_utf8_lossy(&stdout);

        // 对于没有 watchers 的情况, 需要在这里读取输出内容
        let mut out = String::new();
        channel.read_to_string(&mut out)?;
        stdout.to_mut().push_str(&out);

        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr).unwrap();

        channel.wait_close().unwrap();

        let status: i32 = channel.exit_status()?;

        Ok((stdout.to_string(), stderr.to_string(), status))
    }

    pub fn sudo_with_options(&mut self, cmd: &str, options: Option<SudoOptions>) -> Result<(String, String, i32), Box<dyn Error>> {

        let mut opt = match options {
            Some(options) => options,
            None => SudoOptions::new("root", "", "[sudo] password:"),
        };

        if opt.sudo_user == "" {
            opt.sudo_user = "root".to_string();
        }
    
        if opt.sudo_password == "" {
            match self.get_auth() {
                Auth::Password(pwd) => opt.sudo_password = pwd.to_string(),
                Auth::Privatekey(_) => {},
                Auth::Privatekeyfile(_) => {},
            }
        }
    
        if opt.sudo_pattern == "" {
            opt.sudo_pattern = "[sudo] password:".to_string()
        }

        opt.set_watcher(&opt.sudo_pattern.clone(), &opt.sudo_password.clone(), true);

        let cmd = format!("sudo -S -p '{}' -H -u {} /bin/bash -l -c \"cd; {}\"", opt.sudo_pattern, opt.sudo_user, cmd);

        self.run_with_options(&cmd, Some(opt.run))

    }

    pub fn run(&mut self, cmd: &str) -> Result<(String, String, i32), Box<dyn Error>> {
        self.run_with_options(cmd, None)
    }

    pub fn sudo(&mut self, cmd: &str) -> Result<(String, String, i32), Box<dyn Error>> {
        self.sudo_with_options(cmd, None)
    }

    pub fn scp(&mut self, source: &str, target: &str) -> Result<(), Box<dyn Error>> {
        // 上传文件
        let source_file = fs::read(source)?;
        let mut target_file = self.session.scp_send(Path::new(&target), 0o755, source_file.len() as u64, None)?;

        target_file.write(&source_file)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn it_works() {
        let myhost = env::var("MYHOST").expect("$MYHOST is not defined");
        let myport = env::var("MYPORT").expect("$MYPORT is not defined");
        let myusername = env::var("MYUSERNAME").expect("$MYUSERNAME is not defined");
        let mypassword = env::var("MYPASSWORD").expect("$MYPASSWORD is not defined");

        let port: u16 = myport.parse().unwrap();

        let mut conn = Connection::new(myhost, port, myusername, Auth::Password(mypassword)).unwrap();

        let (stdout, stderr, status) = conn.run("ls").unwrap();
        println!("The output of run is: {}, The outerr of run is: {}, The status code of run is: {}, end.", stdout, stderr, status);

        let (stdout, stderr, status) = conn.sudo("ls -l /root/").unwrap();
        println!("The output of sudo is: {}, The outerr of sudo is: {}, The status code of sudo is: {}, end.", stdout, stderr, status);

        conn.scp("/home/xxx/x.xml", "/home/xxx/xx.xml").unwrap();
    }
}

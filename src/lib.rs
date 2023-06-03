use ssh2::{self, Session};
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
// use ssh2::PtyModes;

pub struct Watcher {
    pattern: String,  // 要捕获字符串
    response: String, // 捕获到匹配的字符串出现后, 要输入的内容
    // sentinel: String, // TODO 应答后返回的信息与之匹配, 说明应答失败. 还未实现
    to_upper: bool, // 是否将捕获的字符串和要匹配的字符串全转换成大写再进行比较
}

pub enum Auth {
    Password(String),
    Privatekey(String),
    Privatekeyfile(String),
}

pub struct Connection<'a> {
    host: String,
    port: i32,
    user: &'a str,
    auth: &'a Auth,
    timeout: u32,
    session: ssh2::Session,
    watchers: Vec<Watcher>,
}

impl<'a> Connection<'a> {
    pub fn new(
        host: String,
        port: i32,
        user: &'a str,
        auth: &'a Auth,
    ) -> Result<Connection<'a>, Box<dyn Error>> {
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
            watchers: Vec::new(),
        };

        conn.authenticated(user, auth)?;

        Ok(conn)
    }

    pub fn auth_config(&mut self, user: &str, auth: &Auth) -> Result<(), ssh2::Error> {
        match auth {
            Auth::Password(password) => self.session.userauth_password(user, &password),
            Auth::Privatekey(privatekey) => {
                self.session
                    .userauth_pubkey_memory(user, None, &privatekey, None)
            }
            Auth::Privatekeyfile(privatekey_file) => {
                let privatekey_path = Path::new(&privatekey_file);
                self.session
                    .userauth_pubkey_file(user, None, privatekey_path, None)
            }
        }
    }

    pub fn authenticated(&mut self, user: &str, auth: &Auth) -> Result<(), ssh2::Error> {
        self.auth_config(user, auth)?;
        self.session.authenticated();  // TODO: 这里返回了 bool, 需要处理 false 的情况
        Ok(())
    }

    pub fn set_timeout(&mut self, timeout: u32) {
        self.timeout = timeout
    }

    pub fn set_watcher(&mut self, pattern: String, response: String, to_upper: bool) {
        self.watchers.push(Watcher {
            pattern,
            response,
            to_upper,
        });
    }

    fn run_watcher(
        &mut self,
        channel: &mut ssh2::Channel,
        stdout: &mut Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            let mut buf = [0; 1024];

            let n = channel.read(&mut buf)?;
            if n == 0 {
                break;
            }

            let slice = &buf[0..n];
            stdout.extend_from_slice(slice);

            let s = String::from_utf8_lossy(slice);

            for w in self.watchers.iter() {
                if w.to_upper {
                    if s.to_uppercase().contains(&w.pattern.to_uppercase()) {
                        channel.write_all(format!("{}\n", w.response).as_bytes())?;
                        channel.flush()?;
                    }
                } else {
                    if w.to_upper {
                        if s.contains(&w.pattern) {
                            channel.write_all(format!("{}\n", w.response).as_bytes())?;
                            channel.flush()?;
                        }
                    }
                }
            }

            // match self.channel.read(&mut buf) {
            //     Err(e) => e,
            //     // the channel has closed and we got an EOF
            //     Ok(0) => Ok(()),
            //     // We got some data; try to decode it as utf-8
            //     Ok(n) => {
            //         let slice = &buf[0..n];
            //         stdout.extend_from_slice(slice);

            //         match std::str::from_utf8(slice) {
            //             Err(e) => e,
            //             Ok(s) => {
            //                 for w in self.watchers.iter() {
            //                     if w.to_upper {
            //                         if s.to_uppercase(). contains(w.pattern.to_uppercase()) {
            //                             channel                                            .write_all(format!("{}\n", w.response).as_bytes())?;
            //                             channel.flush()?;
            //                         }
            //                     } else {
            //                         if w.to_upper {
            //                             if s..contains(w.pattern) {
            //                                 channel
            //                                     .write_all(format!("{}\n", w.response).as_bytes())?;
            //                                 channel.flush()?;
            //                             }
            //                     }
            //                 }
            //             }
            //         }
            //     }
            // }
        }
        Ok(())
    }

    pub fn run(&mut self, cmd: &str) -> Result<(String, String), Box<dyn Error>> {
        let mut channel = self.session.channel_session()?;

        // let mut modes = PtyModes::new();
        // modes.set_boolean(ssh2::PtyModeOpcode::TCSANOW, false);
        // modes.set_boolean(ssh2::PtyModeOpcode::ECHO, false);
        // modes.set_u32(ssh2::PtyModeOpcode::TTY_OP_ISPEED, 144000);
        // modes.set_u32(ssh2::PtyModeOpcode::TTY_OP_OSPEED, 144000);
        // channel.request_pty("xterm", Some(modes),Some((80, 40, 0, 0))).unwrap();

        channel.handle_extended_data(ssh2::ExtendedData::Merge)?;
        channel.exec(cmd)?;

        let mut stdout = Vec::new();
        if self.watchers.len() != 0 {
            self.run_watcher(&mut channel, &mut stdout)?
        }

        let mut data = String::from_utf8_lossy(&stdout);

        let mut out = String::new();
        channel.read_to_string(&mut out)?;
        data.to_mut().push_str(&out);
        
        // let mut err = String::new();
        // channel.stderr().read_to_string(&mut err).unwrap();
        
        channel.wait_close().unwrap();
        
        let mut stderr = String::new();
        let status: i32 = channel.exit_status()?;
        if status != 0 {
            stderr = "Incorrect status code".to_string();
        }

        Ok((data.to_string(), stderr))
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn it_works() {
        let myhost = env::var("MYHOST").expect("$HOME is not defined");
        let myport = env::var("MYPORT").expect("$HOME is not defined");
        let myusername = env::var("MYUSERNAME").expect("$HOME is not defined");
        let mypassword = env::var("MYPASSWORD").expect("$HOME is not defined");

        let port: i32 = myport.parse().unwrap();

        let binding = Auth::Password(mypassword);
        let mut conn = Connection::new(myhost, port, &myusername, &binding).unwrap();
        let (s1, s2) = conn.run("ls").unwrap();
        println!("{}, {}", s1, s2);
    }
}

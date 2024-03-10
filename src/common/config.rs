use std::{
    env, 
    net::{AddrParseError, IpAddr, SocketAddr}, 
    num::ParseIntError
};

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Client,
    Server
}

impl Mode {
    fn from(arg: &str) -> Result<Self, ArgError> {
        match arg {
            "client" => Ok(Self::Client),
            "server" => Ok(Self::Server),
            _ => Err(ArgError::ModeIncorrect)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub mode: Mode,
    pub tcp: SocketAddr,
    pub name: String
}

impl Config {
    fn new(mode: Mode, tcp: SocketAddr, name: String) -> Self {
        Config { mode, tcp, name }
    }
}

#[derive(Debug, Clone, Copy, Error)]
pub enum ArgError {
    ModeUnspecified,
    ModeIncorrect,
    AddressUnspecified,
    AddressIncorrect,
    PortUnspecified,
    PortIncorrect,
    NameUnspecified,
}

impl From<AddrParseError> for ArgError {
    fn from(_: AddrParseError) -> Self {
        Self::AddressIncorrect
    }
}

impl From<ParseIntError> for ArgError {
    fn from(_: ParseIntError) -> Self {
        Self::PortIncorrect
    }
}

#[derive(Debug, Clone)]
pub struct Args(Vec<String>);

impl Args {
    pub fn new() -> Self {
        let args = env::args()
            .skip(1)
            .collect::<Vec<_>>();
        
        Args(args)
    }

    pub fn parse(self) -> Result<Config, ArgError> {
        let args = self.0;
        
        let mode = args
            .get(0)
            .ok_or(ArgError::ModeUnspecified)?;

        let mode = Mode::from(mode)?;

        let ip = args
            .get(1)
            .ok_or(ArgError::AddressUnspecified)?
            .parse::<IpAddr>()?;

        let port = args
            .get(2)
            .ok_or(ArgError::PortUnspecified)?
            .parse::<u16>()?;

        let tcp = SocketAddr::new(ip, port);

        let name = args
            .get(3)
            .ok_or(ArgError::NameUnspecified)?
            .to_owned();

        if name.len() == 0 {
            return Err(ArgError::NameUnspecified);
        }

        Ok(Config::new(mode, tcp, name))
    }
}

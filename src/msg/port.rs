use rand::{rng, seq::SliceRandom};
use std::{
    io::{Error, Result},
    net::{IpAddr, SocketAddr, TcpListener, ToSocketAddrs},
    ops::RangeInclusive,
    vec::IntoIter
};

static IANA_USER_PORT_RANGE: RangeInclusive<u16> = 1024..=49151;

pub struct RandomUserPort(RangeInclusive<u16>);

impl RandomUserPort {
    pub fn new() -> Self {
        RandomUserPort(IANA_USER_PORT_RANGE.clone())
    }

    pub fn find() -> Result<u16> {
        // Try binding to 10 ports. If this doesn't work, something else is
        // probably wrong
        for _ in 0..10 {
            if let Ok(res) = TcpListener::bind(Self::new()) {
                return Ok(res.local_addr()?.port());
            }
        }
        Result::Err(Error::from(std::io::ErrorKind::NotFound))
    }
}

impl ToSocketAddrs for RandomUserPort {
    type Iter = IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter> {
        // You might want to use localhost (127.0.0.1) as default IP
        let ip = IpAddr::from([127, 0, 0, 1]);

        // Convert the range into a vector of SocketAddr
        let mut addrs = self
            .0
            .clone()
            .map(|port| SocketAddr::new(ip, port))
            .collect::<Vec<_>>();

        let mut rng = rng();
        addrs.shuffle(&mut rng);

        Ok(addrs.into_iter())
    }
}

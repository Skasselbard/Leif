use futures::sync::mpsc;
use get_broadcasts;
use std::io::{self, Error, ErrorKind, Read, Result};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use tokio::runtime::Runtime;

pub struct Node {
    //task: Future<Item = T>,
}

impl Node {
    pub fn run<F>(&self, task: F)
    where
        F: Future + Send + 'static,
    {
        let broker = get_broker();
        if let Ok(broker) = broker {
            println!("found broker: {}", broker);
            tokio::run(task.then(|_| Ok(())));
        }
        error!("Cannot find broker");
    }
}

fn get_broker() -> Result<SocketAddr> {
    const MAX_DATAGRAM_SIZE: usize = 65_507;
    for broadcast in get_broadcasts().unwrap() {
        // 8207 => BROT(cast)
        // setup socket
        let remote_addr = SocketAddr::new(broadcast, 8207);
        println!("Broadcast to {}", remote_addr);
        //let socket = UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
        let socket = UdpSocket::bind(&"[::]:0".parse().unwrap()).unwrap();
        socket.set_broadcast(true).unwrap();
        //define task
        let request_broker_ip = socket
            .send_dgram("SearchingBroker", &remote_addr)
            .and_then(|(socket, _)| socket.recv_dgram(vec![0u8; MAX_DATAGRAM_SIZE]))
            .deadline(Instant::now() + Duration::from_secs(2))
            .map_err(|_e| Error::new(ErrorKind::TimedOut, "No answer in this net"));
        // execute task
        let mut runtime = Runtime::new().unwrap();
        let result = runtime.block_on(request_broker_ip);
        match result {
            Ok((socket, data, len, remote_addr)) => return Ok(remote_addr),
            Err(e) => {}
        }
    }
    Err(Error::new(ErrorKind::NotFound, "Unable to find broker"))
}

fn run2() {
    // let mut args = env::args().skip(1).collect::<Vec<_>>();
    // let tcp = match args.iter().position(|a| a == "--udp") {
    //     Some(i) => {
    //         args.remove(i);
    //         false
    //     }
    //     None => true,
    // };

    // let addr = args.first()
    //     .unwrap_or_else(|| panic!("this program requires at least one argument"));
    // let addr = addr.parse::<SocketAddr>().unwrap();

    // let (stdin_tx, stdin_rx) = mpsc::channel(0);
    // thread::spawn(|| read_stdin(stdin_tx));
    // let stdin_rx = stdin_rx.map_err(|_| panic!());
    // let stdout = if tcp {
    //     tcp::connect(&addr, Box::new(stdin_rx))
    // } else {
    //     udp::connect(&addr, Box::new(stdin_rx))
    // };

    // let mut out = io::stdout();

    // tokio::run({
    //     stdout
    //         .for_each(move |chunk| out.write_all(&chunk))
    //         .map_err(|e| println!("error reading stdout; error = {:?}", e))
    // });
}

mod codec {
    use bytes::{BufMut, BytesMut};
    use std::io;
    use tokio_codec::{Decoder, Encoder};

    pub struct Bytes;

    impl Decoder for Bytes {
        type Item = BytesMut;
        type Error = io::Error;

        fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<BytesMut>> {
            if buf.len() > 0 {
                let len = buf.len();
                Ok(Some(buf.split_to(len)))
            } else {
                Ok(None)
            }
        }
    }

    impl Encoder for Bytes {
        type Item = Vec<u8>;
        type Error = io::Error;

        fn encode(&mut self, data: Vec<u8>, buf: &mut BytesMut) -> io::Result<()> {
            buf.put(&data[..]);
            Ok(())
        }
    }
}

mod tcp {
    use tokio;
    use tokio::net::TcpStream;
    use tokio::prelude::*;
    use tokio_codec::Decoder;

    use bytes::BytesMut;
    use node::codec::Bytes;

    use std::io;
    use std::net::SocketAddr;

    pub fn connect(
        addr: &SocketAddr,
        stdin: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>,
    ) -> Box<Stream<Item = BytesMut, Error = io::Error> + Send> {
        let tcp = TcpStream::connect(addr);

        Box::new(tcp.map(move |stream| {
            let (sink, stream) = Bytes.framed(stream).split();

            tokio::spawn(stdin.forward(sink).then(|result| {
                if let Err(e) = result {
                    panic!("failed to write to socket: {}", e)
                }
                Ok(())
            }));

            stream
        }).flatten_stream())
    }
}

mod udp {
    use std::io;
    use std::net::SocketAddr;

    use bytes::BytesMut;
    use tokio;
    use tokio::net::{UdpFramed, UdpSocket};
    use tokio::prelude::*;

    use node::codec::Bytes;

    pub fn connect(
        &addr: &SocketAddr,
        stdin: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>,
    ) -> Box<Stream<Item = BytesMut, Error = io::Error> + Send> {
        let addr_to_bind = if addr.ip().is_ipv4() {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let udp = UdpSocket::bind(&addr_to_bind).expect("failed to bind socket");

        let (sink, stream) = UdpFramed::new(udp, Bytes).split();

        let forward_stdin = stdin
            .map(move |chunk| (chunk, addr))
            .forward(sink)
            .then(|result| {
                if let Err(e) = result {
                    panic!("failed to write to socket: {}", e)
                }
                Ok(())
            });

        let receive = stream.filter_map(move |(chunk, src)| {
            if src == addr {
                Some(chunk.into())
            } else {
                None
            }
        });

        Box::new(
            future::lazy(|| {
                tokio::spawn(forward_stdin);
                future::ok(receive)
            }).flatten_stream(),
        )
    }
}

fn read_stdin(mut tx: mpsc::Sender<Vec<u8>>) {
    let mut stdin = io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf) {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx = match tx.send(buf).wait() {
            Ok(tx) => tx,
            Err(_) => break,
        };
    }
}

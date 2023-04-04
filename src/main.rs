use std::{
    collections::HashMap,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    ops::{Add, Div},
    sync::mpsc::channel,
    sync::mpsc::Sender,
    time::{Duration, SystemTime},
    vec,
};

use clap::Parser;
#[derive(Parser, Debug)]
struct App {
    #[arg(long, short, default_value_t = 16)]
    count: usize,
    #[arg(long, short, default_value_t = 10)]
    timeout: u64,
    #[arg(long, short, default_value_t = 80)]
    port: u16,
    #[arg(long, default_value_t = 1)]
    threads: usize,
    #[arg(value_name = "HOSTNAME")]
    hosts: Vec<String>,
}

#[derive(Clone)]
struct PingUpdate {
    hostname: String,
    sequence_number: usize,
    result: Result<Duration, String>,
}

///Will send a SYN TCP message to the "hostname" then time the time until it recives a ACK message
/// It will then return an update via the "return_channel" provided note all connection are subject
/// to the "timeout" and it will repeat this as many times as spcified by "number_of_messages"
fn ping(
    hostname: String,
    number_of_message: usize,
    timeout: Duration,
    return_channel: Sender<PingUpdate>,
) {
    for i in 0..number_of_message {
        let sockaddr: SocketAddr;
        match hostname.to_socket_addrs() {
            Ok(data) => {
                let addrs: Vec<SocketAddr> = data.collect();
                match addrs.get(0) {
                    Some(addr) => {
                        sockaddr = addr.clone();
                    }
                    None => {
                        return_channel
                            .send(PingUpdate {
                                hostname: hostname.clone(),
                                sequence_number: i,
                                result: Err("Unable to resolve DNS name".to_string()),
                            })
                            .unwrap();
                        continue;
                    }
                }
            }
            Err(e) => {
                return_channel
                    .send(PingUpdate {
                        hostname: hostname.clone(),
                        sequence_number: i,
                        result: Err(e.to_string()),
                    })
                    .unwrap();
                continue;
            }
        }
        let duration = SystemTime::now();
        let update = match TcpStream::connect_timeout(&sockaddr, timeout) {
            Ok(_) => match duration.elapsed() {
                Ok(time) => PingUpdate {
                    hostname: hostname.clone(),
                    sequence_number: i,
                    result: Ok(time),
                },
                Err(e) => PingUpdate {
                    hostname: hostname.clone(),
                    sequence_number: i,
                    result: Err(e.to_string()),
                },
            },
            Err(e) => PingUpdate {
                hostname: hostname.clone(),
                sequence_number: i,
                result: Err(e.to_string()),
            },
        };
        return_channel.send(update).unwrap();
    }
}

/// returns a string containing strings of length "size"
fn gen_filler(size: usize) -> String {
    let mut output_filler = String::new();
    for _ in 0..size {
        output_filler.push(' ');
    }
    return output_filler;
}

/// returns a string for spacing out the diffrence between the max count and the current sequence number
fn gen_spacer(count: usize, seq_num: usize) -> String {
    let spacer_size =
        (count.checked_ilog10().unwrap_or(0) + 1) - seq_num.checked_ilog10().unwrap_or(0);
    let mut buffer = String::new();
    for _ in 0..spacer_size {
        buffer.push(' ');
    }
    return buffer;
}

fn main() -> Result<(), String> {
    let app = App::parse();
    if app.hosts.len() < 1 {
        return Err("No Endpoints given".to_string());
    }
    let (tx, rx) = channel();
    let thread_pool = threadpool::ThreadPool::new(app.threads);
    let mut max_hostname_len: usize = 0;
    for host in app.hosts.clone().iter_mut() {
        if !host.contains(':') {
            host.clear();
            host.push_str(format!("{}:{}", host, app.port).as_str());
        }
        if max_hostname_len < host.len() {
            max_hostname_len = host.len();
        }
        let return_channel = tx.clone();
        let target_host = host.clone();
        let number_of_messages = app.count.clone();
        let timeout = app.timeout.clone();
        thread_pool.execute(move || {
            ping(
                target_host,
                number_of_messages,
                Duration::from_secs(timeout),
                return_channel,
            );
        })
    }

    let mut results: HashMap<String, Vec<PingUpdate>> = HashMap::new();
    for update in rx.iter().take(app.count * app.hosts.len()) {
        match results.get_mut(&update.hostname) {
            Some(updates) => {
                updates.push(update.clone());
            }
            None => {
                results.insert(update.hostname.clone(), vec![update.clone()]);
            }
        }
        let output_text = format!(
            "{}:{}{} {}",
            update.sequence_number,
            gen_spacer(app.count, update.sequence_number),
            update.hostname,
            gen_filler(max_hostname_len - update.hostname.len())
        );
        match update.result {
            Ok(dur) => {
                println!("{} {}ms", output_text, dur.as_millis())
            }
            Err(err) => {
                println!("{} ERROR: {}", output_text, err)
            }
        }
    }
    for (hostname, updates) in results.iter() {
        let packets_sent: usize = updates.len();
        let mut packets_recived: usize = 0;
        let mut max_rtt: Duration = Duration::new(0, 0);
        let mut min_rtt: Duration = Duration::new(app.timeout, 0);
        let mut average: Duration = Duration::new(0, 0);
        for update in updates {
            match update.result {
                Ok(duration) => {
                    packets_recived += 1;
                    if max_rtt < duration {
                        max_rtt = duration;
                    }
                    if min_rtt > duration {
                        min_rtt = duration;
                    }
                    average = average.add(duration);
                }
                Err(_) => {}
            }
        }
        print!(
            "Ping statistics for {}:\n\tPackets: sent {}, Recived {}, Lost {} ({}% loss).\n\tRound Trip times: Minimum {}ms, Maximum {}ms, Average {}ms\n",
            hostname,
            packets_sent,
            packets_recived,
            packets_sent - packets_recived,
            100 - (packets_recived / packets_sent) * 100,
            min_rtt.as_millis(),
            max_rtt.as_millis(),
            average.as_millis().div(updates.len() as u128)
        );
    }

    return Ok(());
}

#[cfg(test)]
mod test {
    use std::{sync::mpsc::channel, time::Duration};

    use crate::{gen_filler, gen_spacer, ping};

    #[test]
    fn test_get_spacer_size() {
        let test_cases = [
            ((1, 6), " ".to_string()),
            ((1, 100000), "      ".to_string()),
            ((10, 1), "".to_string()),
        ];
        for test_case in test_cases {
            let ((seq_num, count), expected_result) = test_case;
            assert_eq!(gen_spacer(count, seq_num), expected_result)
        }
    }

    #[test]
    fn test_gen_filler() {
        let test_cases = [
            (1, " ".to_string()),
            (0, "".to_string()),
            (10, "          ".to_string()),
        ];
        for test_case in test_cases {
            let (test, expected_result) = test_case;
            assert_eq!(gen_filler(test), expected_result)
        }
    }
    #[test]
    fn test_ping() {
        let test_cases = [("www.google.com:443".to_string(), 1, Duration::from_secs(1))];
        for test_case in test_cases {
            let (hostname, count, timeout) = test_case;
            let (tx, rx) = channel();
            ping(hostname.clone(), count, timeout, tx);

            for update in rx.iter().take(count) {
                assert_eq!(update.hostname, hostname);
                if let Err(e) = update.result {
                    panic!("{}", e);
                }
            }
        }
        let neg_test_cases = [
            ("127.0.0.1:16000".to_string(), 1, Duration::from_secs(1)),
            ("www.google.com:443".to_string(), 1, Duration::from_secs(0)),
        ];
        for test_case in neg_test_cases {
            let (hostname, count, timeout) = test_case;
            let (tx, rx) = channel();
            ping(hostname.clone(), count, timeout, tx);

            for update in rx.iter().take(count) {
                assert_eq!(update.hostname, hostname);
                if let Ok(_) = update.result {
                    panic!("Expected to not be able to connect to this port in the given timeout range.");
                }
            }
        }
    }
}

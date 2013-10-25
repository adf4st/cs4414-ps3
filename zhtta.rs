//
// zhtta.rs
//
// Running on Rust 0.8
//
// Code for PS3
// 
// Note: it would be very unwise to run this server on a machine that is
// on the Internet and contains any sensitive files!
//
// University of Virginia - cs4414 Fall 2013
// Daniel Nizri, Renee Seaman, and Alex Fabian
// Version 0.4

extern mod extra;

use std::rt::io::*;
use std::rt::io::net::ip::SocketAddr;
use std::io::println;
use std::cell::Cell;
use std::{os, str, io};
use extra::arc;
use std::comm::*;
use std::run;

static PORT:    int = 4414;
static IP: &'static str = "127.0.0.1";
//static mut visitor_count: uint = 0;

struct sched_msg {
    stream: Option<std::rt::io::net::tcp::TcpStream>,
    filepath: ~std::path::PosixPath,
    file_size: uint
}

fn main() {
    let req_vec: ~[sched_msg] = ~[];
    let shared_req_vec = arc::RWArc::new(req_vec);
    let add_vec = shared_req_vec.clone();
    let take_vec = shared_req_vec.clone();
    let visitor_count : uint = 0;
    let shared_visitor_count = arc::RWArc::new(visitor_count);
    
    let (port, chan) = stream();
    let chan = SharedChan::new(chan);
    
    // dequeue file requests, and send responses.
    // FIFO
    do spawn {
        let (sm_port, sm_chan) = stream();
        
        // a task for sending responses.
        do spawn {
            loop {
                let mut tf: sched_msg = sm_port.recv(); // wait for the dequeued request to handle
                match io::read_whole_file_str(tf.filepath) { // killed if file size is larger than memory size.
                    Ok(file_data) => {
                        println(fmt!("begin serving file [%?]", tf.filepath));
                        // A web server should always reply a HTTP header for any legal HTTP request.
                        tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n".as_bytes());
                        //application/octet-stream
                        
                        let mut file_data_vec = ~[file_data.clone()];
                        
                        while(file_data_vec[0].contains("<!--#exec cmd=\"")) {
                            let file_data_clone = file_data_vec[0].clone();
                            let start_pos_ssi = file_data_clone.find_str("<!--#exec cmd=\"");
                            match start_pos_ssi {
                                Some(start_ssi) => {
                                    let end_pos_ssi = file_data_clone.find_str("\" -->");
                                    match end_pos_ssi {
                                        Some(end_ssi) => {
                                            let start_pos_cmd = start_ssi + 15;
                                            let end_pos_cmd = end_ssi;
                                            let cmd = file_data_clone.slice(start_pos_cmd, end_pos_cmd);
                                            
                                            let mut argv: ~[~str] = cmd.split_iter(' ').filter_map(|x| 
                                                if x != "" {
                                                    Some(x.to_owned())
                                                }
                                                else {
                                                    None
                                                }).to_owned_vec();

                                            if argv.len() > 0 {
                                                let program = argv.remove(0);
                                                let output = str::from_utf8(run::process_output(program, argv).output);
                                                file_data_vec[0] = file_data_clone.slice(0, start_ssi).clone().to_owned().append(output).append(file_data_clone.slice(end_ssi + 5, file_data_clone.len()));
                                            }
                                        }
                                        None      => { println("Not found") }
                                    }
                                }
                                None      => { }
                            }
                        }
                        
                        tf.stream.write(file_data_vec[0].as_bytes());
                        println(fmt!("finish file [%?]", tf.filepath));
                    }
                    Err(err) => {
                        println(err);
                    }
                }
            }
        }
        
        loop {
            port.recv(); // wait for arrving notification
            do take_vec.write |vec| {
                if ((*vec).len() > 0) {
                    // LIFO didn't make sense in service scheduling, so we modify it as FIFO by using shift_opt() rather than pop().
                    let tf_opt: Option<sched_msg> = (*vec).shift_opt();
                    let tf = tf_opt.unwrap();
                    println(fmt!("shift from queue, size: %ud", (*vec).len()));
                    sm_chan.send(tf); // send the request to send-response-task to serve.
                }
            }
        }
    }

    let ip = match FromStr::from_str(IP) { Some(ip) => ip, 
                                           None => { println(fmt!("Error: Invalid IP address <%s>", IP));
                                                     return;},
                                         };
    
    let socket = net::tcp::TcpListener::bind(SocketAddr {ip: ip, port: PORT as u16});
    
    println(fmt!("Listening on %s:%d ...", ip.to_str(), PORT));
    let mut acceptor = socket.listen().unwrap();
    
    for stream in acceptor.incoming() {
        let stream = Cell::new(stream);
        
        // Start a new task to handle the each connection
        let child_chan = chan.clone();
        let child_add_vec = add_vec.clone();
        let (port2, chan2) = std::comm::stream();
        chan2.send(shared_visitor_count.clone());
        do spawn {
            let visitor_count = port2.recv();
            visitor_count.write(|count| {
                *count += 1;
            });
            
            let mut stream = stream.take();
            let mut buf = [0, ..500];
            stream.read(buf);
            let request_str = str::from_utf8(buf);
            
            let req_group : ~[&str]= request_str.splitn_iter(' ', 3).collect();
            if req_group.len() > 2 {
                let path = req_group[1];
                println(fmt!("Request for path: \n%?", path));
                
                let file_path = ~os::getcwd().push(path.replace("/../", ""));
                if !os::path_exists(file_path) || os::path_is_dir(file_path) {
                    visitor_count.read(|val| {
                        println(fmt!("Request received:\n%s", request_str));
                        let response: ~str = fmt!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n
                             <doctype !html><html><head><title>Hello, Rust!</title>
                             <style>body { background-color: #111; color: #FFEEAA }
                                    h1 { font-size:2cm; text-align: center; color: black; text-shadow: 0 0 4mm red}
                                    h2 { font-size:2cm; text-align: center; color: black; text-shadow: 0 0 4mm green}
                             </style></head>
                             <body>
                             <h1>Greetings, Krusty!</h1>
                             <h2>Visitor count: %u</h2>
                             </body></html>\r\n", *val);

                        stream.write(response.as_bytes());
                    });
                }
                else {
                    // Requests scheduling
                    let mut file_size = 0;
                    let reader : Result<@io::Reader, ~str> = io::file_reader(file_path);
                    match reader {
                        Ok(reader) => {
                            reader.seek(0, io::SeekEnd);
                            file_size = reader.tell();
                        },
                        Err(msg) => {
                            println(msg);
                        },
                    }
                    
                    let msg: sched_msg = sched_msg{stream: stream, filepath: file_path.clone(), file_size: file_size};
                    let (sm_port, sm_chan) = std::comm::stream();
                    sm_chan.send(msg);
                    
                    let ip_str = ip.to_str().clone();
                    let mut is_ip_preferred = false;
                    
                    if ip_str.slice(0, 8) == "128.143." || ip_str.slice(0, 7) == "137.54." || ip_str.slice(0, 8) == "192.168." {
                        is_ip_preferred = true;
                    }
                    
                    do child_add_vec.write |vec| {
                        let msg = sm_port.recv();
                        if is_ip_preferred {
                            (*vec).insert(0, msg); // enqueue new preferred request.
                        }
                        else {
                            let mut insert_pos = 0;
                            for queued_msg in vec.iter() {
                                if msg.file_size >= queued_msg.file_size {
                                    insert_pos += 1;
                                }
                                else {
                                    break;
                                }
                            }
                            (*vec).insert(insert_pos, msg); // enqueue new request.
                        }
                        println("add to queue");
                    }
                    child_chan.send(""); //notify the new arriving request.
                    println(fmt!("get file request: %?", file_path));
                }
            }
            println!("connection terminates")
        }
    }
}

# TCPing 

a simple multithreaded ping utility. this ping utility is written for generic
TCP Sockets using the sync ack messges of the TCP protocol.

## Why reimplment ping

1. I work with a lot of wreird of OS's that don't allways reply to ICMP packets 
    but do have network services running on them e.g. VxWorks, LynxOS
2. Normal Ping can sometimes be blocked by overly agressive firewalls

## Yeah but this is hardly the first TCP ping implemetion why bother

Well thats true and if you want a more portable implemetnion of TCP Ping then 
the python implemention is completly valid and fine. However, by implmenting it
myself I can implement features that only i need and I can fix more easily when
it breaks (as all software allways does).

## Examples 

- This uses the same idea as every other ping implemention 

```bash
tcping www.google.com
```

- We do allow for multiple hosts

```bash
tcping www.google.com www.bing.com www.yahoo.com
```

- We even allow for pinging multiple hosts on multiple threads

```bash
tcping --threads 2  www.google.com www.bing.com www.yahoo.com www.ebay.com
```

- you can even set the port that is used

```bash
tcping www.google.com:443
```

for all features that are available please use the --help flag when running the
application.

### Warning 

Alot of sevices will limit the number of TCP connections that you can open and
close to the same server in a given time so therefore this server might start 
rejecting connnetion with no fault of the programme.

## Theory

The TCP Protocol specifies that to open the connection you must:

1. Send a SYN message this provides a starting sequence number
2. IF the connection is accepted then the server will reply with a ACK message
   and the SYN value and sequence nubmer of the SYN Message that was sent.

![TCP Connection Explainer image](https://upload.wikimedia.org/wikipedia/commons/thumb/5/55/TCP_CLOSE.svg/1920px-TCP_CLOSE.svg.png)
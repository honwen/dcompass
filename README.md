# dcompass

![Automated build](https://github.com/LEXUGE/dcompass/workflows/Build%20dcompass%20on%20various%20targets/badge.svg)
[![Join telegram channel](https://badges.aleen42.com/src/telegram.svg)](https://t.me/dcompass_channel)  
A high-performance DNS server with flexible routing scheme and customized plugins.  
[中文版](README-CN.md)

Below is an example of using GeoIP to mitigate DNS pollution

```yaml
table:
  start:
    - query: domestic
    - check_secure
  check_secure:
    if: "!geoip(codes: [\"CN\"])"
    then:
      - query: secure
      - end
```

# Features

- Fast (~50000 qps in wild where upstream perf is about the same)
- Flexible routing rules that are easy to compose and maintain
- Built-in plugins to manipulate your DNS queries and responses (you can also add or tailor them to your need)
- Fearless hot switch between network environments
- DNS over HTTPS support
- Written in pure Rust

# Notice
**[2021-9-16] Expression Engine and breaking changes**  
dcompass is now equipped with an expression engine which let you easily and freely compose logical expressions with existing matchers. This enables us to greatly improve config readablity and versatility. However, all existing config files involving if rule block are no longer working. Please see examples to migrate.

**[2021-07-28] 2x faster and breaking changes**  
We adopted a brand new bare metal DNS library `domain` which allows us to manipulate DNS messages without much allocation. This adoption significantly improves the memory footprint and throughput of dcompass. Due to this major refactorization, DoT/TCP/zone protocol are temporarily unavailable, however, UDP and DoH connections are now blazing fast. We will gradually put back those protocols.

# Usages

```
dcompass -c path/to/config.json # Or YAML
```

Or you can simply run `dcompass` from the folder where your configuration file named `config.yml` resides.  
You can also validate your configuration

```
dcompass -c path/to/config.json -v
```

# Quickstart

See [example.yaml](configs/example.yaml)

# Configuration

Configuration file contains different fields:

- `verbosity`: Log level filter. Possible values are `trace`, `debug`, `info`, `warn`, `error`, `off`.
- `address`: The address to bind on.
- `table`: A routing table composed of `rule` blocks. The table cannot be empty and should contains a single rule named with `start`. Each rule contains `tag`, `if`, `then`, and `else`. Latter two of which are of the form `(action1, action 2, ... , next)` (you can omit the action and write ONLY `(next)`), which means take the actions first and goto the next rule with the tag specified.
- `upstreams`: A set of upstreams. `timeout` is the time in seconds to timeout, which takes no effect on method `Hybrid` (default to 5). `tag` is the name of the upstream. `methods` is the method for each upstream.

Different actions:

- `blackhole`: Set response with a SOA message to curb further query. It is often used accompanied with `qtype` matcher to disable certain types of queries.
- `query(tag, cache policy)`: Send query via upstream with specified tag. Configure cache policy with one of the three levels: `disabled`, `standard`, `persistent`. See also [example](configs/query_cache_policy.yaml).

Different matchers: (More matchers to come)

- `any`: Matches anything.
- `domain(list of file paths or query name)`: Matches domain in specified domain lists. Supports gzip, lzma, and bzip2.
- `qtype(list of record types)`: Matches record type specified.
- `geoip(codes: list of country codes, path: optional path to the mmdb database file)`: If there is one or more `A` or `AAAA` records at the current state and the first of which has got a country code in the list specified, then it matches, otherwise it always doesn't match.
- `ipcidr(list of files that contain CIDR entries)`: Same as `geoip`, but it instead matches on CIDR. Supports gzip, lzma, and bzip2.
- `header(cond: opcode|rcode|bits, query: bool)`: Matches the condition on query message header or response message header depending on the second option

Different querying methods:

- `https`: DNS over HTTPS querying methods. `uri` is the remote server address in the form like `https://cloudflare-dns.com/dns-query`. `addr` is the server IP address (both IPv6 and IPv4) are accepted. HTTP and SOCKS5 proxies are also accepted on establishing connections via `proxy`, whose format is like `socks5://[user:[passwd]]@[ip:[port]]`.
- `tls`: [CURRENTLY UNSUPPORTED] DNS over TLS querying methods. `no_sni` means don't send SNI (useful to counter censorship). `name` is the TLS certification name of the remote server. `addr` is the remote server address.
- `udp`: Typical UDP querying method. `addr` is the remote server address.
- `hybrid`: Race multiple upstreams together. the value of which is a set of tags of upstreams. Note, you can include another `hybrid` inside the set as long as they don't form chain dependencies, which is prohibited and would be detected by `dcompass` in advance.
- `zone`: [CURRENTLY UNSUPOORTED] use local DNS zone file to provide customized responses. See also [zone config example](configs/success_zone.yaml)

See [example.yaml](configs/example.yaml) for a pre-configured out-of-box anti-pollution configuration (Only works with `full` or `cn` version, to use with `min`, please provide your own database).

# Packages

You can download binaries at [release page](https://github.com/LEXUGE/dcompass/releases).

1. GitHub Action build is set up `x86_64`, `i686`, `arm`, and `mips`. Check them out on release page!
2. NixOS package is available at this repo as a flake. Also, for NixOS users, a NixOS modules is provided with systemd services and easy-to-setup interfaces in the same repository where package is provided.

```
└───packages
    ├───aarch64-linux
    │   ├───dcompass-cn: package 'dcompass-cn-git'
    │   └───dcompass-maxmind: package 'dcompass-maxmind-git'
    ├───i686-linux
    │   ├───dcompass-cn: package 'dcompass-cn-git'
    │   └───dcompass-maxmind: package 'dcompass-maxmind-git'
    ├───x86_64-darwin
    │   ├───dcompass-cn: package 'dcompass-cn-git'
    │   └───dcompass-maxmind: package 'dcompass-maxmind-git'
    └───x86_64-linux
        ├───dcompass-cn: package 'dcompass-cn-git'
        └───dcompass-maxmind: package 'dcompass-maxmind-git'
```

cache is available at [cachix](https://dcompass.cachix.org), with public key `dcompass.cachix.org-1:uajJEJ1U9uy/y260jBIGgDwlyLqfL1sD5yaV/uWVlbk=` (`outputs.publicKey`).

# Benchmark

Mocked benchmark (server served on local loopback):

```
Gnuplot not found, using plotters backend
non_cache_resolve       time:   [20.548 us 20.883 us 21.282 us]
                        change: [-33.128% -30.416% -27.511%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 11 outliers among 100 measurements (11.00%)
  6 (6.00%) high mild
  5 (5.00%) high severe

cached_resolve          time:   [2.6429 us 2.6493 us 2.6566 us]
                        change: [-90.684% -90.585% -90.468%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 2 outliers among 100 measurements (2.00%)
  1 (1.00%) high mild
  1 (1.00%) high severe
```

# TODO-list

- [ ] Support multiple inbound servers with different types like `DoH`, `DoT`, `TCP`, and `UDP`.
- [ ] RESTful API and web dashboard
- [x] Expression engine and function style matcher syntax
- [x] IP-CIDR matcher for both source address and response address
- [x] GeoIP matcher for source address
- [x] Custom response action

# RFC compliance
dcompass should be somewhere between a DNS resolver and DNS forwarder. Currently we don't fully comply RFC on some corner cases. And it is also not clear whether we should correct the incoming malformatted DNS requests. **If you feel a particular compliance to RFC is necessary, please file an issue and we are willing to help!**

# License

All three components `dmatcher`, `droute`, `dcompass` are licensed under GPLv3+.
`dcompass` with `geoip` feature gate enabled includes GeoLite2 data created by MaxMind, available from <a href="https://www.maxmind.com">https://www.maxmind.com</a>.

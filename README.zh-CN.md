<!-- ─── Language Switch & ToC (top-right) ─────────────────────────── -->
<div align="right">

<a href="README.md">🇺🇸 English</a> ·
<span style="color:#999;">🇨🇳 中文</span> &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; 目录↗️

</div>

<h1 align="center"><code>tcping-rs</code></h1>

<p align=center>🛠️ tcping-rs: 检测TCP端口连通性和延迟的工具。</p>

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/tcping.svg)](https://crates.io/crates/tcping)&nbsp;
[![Repo Size](https://img.shields.io/github/repo-size/lvillis/tcping-rs?color=328657)](https://github.com/lvillis/tcping-rs)&nbsp;
[![CI](https://github.com/lvillis/tcping-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/lvillis/tcping-rs/actions)&nbsp;
[![Docker Pulls](https://img.shields.io/docker/pulls/lvillis/tcping?style=flat-square)](https://hub.docker.com/r/lvillis/tcping)&nbsp;
[![Image Size](https://img.shields.io/docker/image-size/lvillis/tcping/latest?style=flat-square)](https://hub.docker.com/r/lvillis/tcping)&nbsp;
[![Say Thanks](https://img.shields.io/badge/Say%20Thanks-!-1EAEDB.svg)](mailto:lvillis@outlook.com?subject=Thanks%20for%20tcping-rs!)

</div>

---

## ✨ 优势

| 亮点               | 描述                                                                |
|--------------------|---------------------------------------------------------------------|
| **纯Rust实现**     | 单文件可执行程序，无需额外运行时依赖                                |
| **免ICMP协议**     | 在传统`ping`被屏蔽的场景仍可用，仅依靠TCP握手检测                   |
| **跨平台**         | 支持Linux、macOS、Windows、*BSD及所有Rust T1级支持的平台            |
| **多种模式**       | `-t` 持续运行, `-c` 指定次数, `-e` 支持提前退出                     |
| **机器可读输出**   | `-o` 支持 JSON(NDJSON) / CSV / Markdown，适用于脚本与监控           |
| **抖动统计**       | `-j` 输出每次探测抖动，并在汇总中给出 p95                           |
| **Docker镜像**     | 提供多架构镜像(`amd64` / `arm64`)，适配CI/CD流水线与Kubernetes任务  |


## 用法

```bash
tcping <host:port> [-c count] [-t] [-e] [-j] [-o mode] [--timestamp[=format] | -D] [--timeout-ms ms]
```

参数:

- `host:port` 要检测的主机和端口
- `-c count` 指定检测次数(默认: 4)
- `-t` 开启持续检测
- `-e` 目标机器握手成功后立即退出
- `-j` 开启抖动输出（每次探测 + 汇总 p95）
- `-o mode` 设置输出格式 (`normal`, `json`, `csv`, `md`, `color`)
- `--timestamp[=format]` 为每条 probe 和 summary 记录附加时间戳；默认 `iso8601`，`--date` 为别名
- `-D` 是 `--timestamp unix` 的简写
- `--timeout-ms` 单次探测超时时间(毫秒，默认: 2000)
- `-h` 打印帮助信息
- `-V` 打印程序版本

## 示例

```bash
$ tcping github.com:443

Resolved github.com -> 140.82.113.4  (DNS system default)  in 0.9340 ms

Probing 140.82.113.4:443/tcp - open - 12.7510 ms
Probing 140.82.113.4:443/tcp - open - 12.4270 ms
Probing 140.82.113.4:443/tcp - open - 11.4410 ms
Probing 140.82.113.4:443/tcp - open - 12.7510 ms

--- 140.82.113.4:443 tcping statistics ---
4 probes sent, 4 successful, 0.00% packet loss
Round-trip min/avg/max = 11.4410/12.3425/12.7510 ms
Address resolved in 0.9340 ms
```

```bash
$ tcping github.com:443 --timestamp -c 2

Resolved github.com -> 140.82.113.4  (DNS system default)  in 0.9340 ms

[2026-04-08T01:15:57.952Z] Probing 140.82.113.4:443/tcp - open - 12.7510 ms
[2026-04-08T01:15:58.954Z] Probing 140.82.113.4:443/tcp - open - 12.4270 ms

[2026-04-08T01:15:58.954Z] --- 140.82.113.4:443 tcping statistics ---
2 probes sent, 2 successful, 0.00% packet loss
Round-trip min/avg/max = 12.4270/12.5890/12.7510 ms
Address resolved in 0.9340 ms
```

## 输出格式

- `-o json`: NDJSON（每行一个 JSON 对象），通过 `schema=tcping.v1` 和 `record=probe|summary` 区分记录类型
- `-o csv`: 单一 CSV 输出流（带表头），通过 `schema=tcping.v1` 和 `record=probe|summary` 区分记录类型
- 开启 `--timestamp` 或 `-D` 后，JSON 和 CSV 会升级为 `schema=tcping.v2`，并为每条 `probe` / `summary` 记录增加 `timestamp`（RFC 3339 UTC）和 `timestamp_unix_ms` 字段
- 面向终端的输出（`normal`、`color`、`md`）直接使用所选样式：`iso8601` 输出毫秒精度的 RFC 3339 UTC，`unix` 输出 `秒.毫秒`

## 安装指南

### 下载发行版

从[版本发布页](https://github.com/lvillis/tcping-rs/releases)获取预编译的二进制文件

* 访问[版本发布页](https://github.com/lvillis/tcping-rs/releases)。
* 下载适用于您操作系统的对应二进制文件。
* 解压可执行文件，将其放置在系统PATH环境变量包含的目录中。

### 使用Docker

使用Docker镜像运行`tcping-rs`:

```shell
docker run --rm docker.io/lvillis/tcping:latest <host:port> [options]

```

## 关于

该工具通过TCP握手测量服务器延迟。采用Rust语言开发，并集成 clap 进行命令行解析，使用 Tokio 实现异步循环与计时器。
